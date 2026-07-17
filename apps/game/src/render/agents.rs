use std::borrow::Borrow;
use std::collections::HashMap;
use std::time::Duration as StdDuration;

use geom::{Circle, Pt2D, QuadTree, Time};
use instant::Instant;
use map_gui::colors::ColorScheme;
use map_gui::options::Options;
use map_model::{Map, Traversable};
use sim::{AgentID, Sim, UnzoomedAgent, VehicleType};
use widgetry::{Color, Drawable, GeomBatch, GfxCtx, Panel, Prerender};

use crate::render::{
    draw_vehicle, unzoomed_agent_radius, DrawPedCrowd, DrawPedestrian, GameRenderable,
};

const UNZOOMED_AGENT_REFRESH_INTERVAL: StdDuration = StdDuration::from_secs(2);

fn should_refresh_unzoomed_agents(is_dragging: bool, has_cached_agents: bool) -> bool {
    !is_dragging || !has_cached_agents
}

fn should_recalculate_unzoomed_agents(
    cache_is_missing: bool,
    sim_time_changed: bool,
    filters_changed: bool,
    force_refresh: bool,
    elapsed_since_refresh: Option<StdDuration>,
) -> bool {
    if cache_is_missing || filters_changed {
        return true;
    }
    if !sim_time_changed {
        return false;
    }
    force_refresh
        || elapsed_since_refresh
            .map(|elapsed| elapsed >= UNZOOMED_AGENT_REFRESH_INTERVAL)
            .unwrap_or(true)
}

pub struct AgentCache {
    /// This is controlled almost entirely by the minimap panel. It has no meaning in edit mode.
    pub unzoomed_agents: UnzoomedAgents,

    // This time applies to agents_per_on. unzoomed has its own possibly separate Time!
    time: Option<Time>,
    agents_per_on: HashMap<Traversable, Vec<Box<dyn GameRenderable>>>,
    // when either of (time, unzoomed agent filters) change, recalculate (a quadtree of all agents,
    // draw all agents)
    unzoomed: Option<(Time, UnzoomedAgents, QuadTree<AgentID>, Drawable)>,
    unzoomed_last_refresh: Option<Instant>,
}

impl AgentCache {
    pub fn new() -> AgentCache {
        AgentCache {
            unzoomed_agents: UnzoomedAgents::new(),
            time: None,
            agents_per_on: HashMap::new(),
            unzoomed: None,
            unzoomed_last_refresh: None,
        }
    }

    pub fn get(&self, on: Traversable) -> Vec<&dyn GameRenderable> {
        self.agents_per_on[&on]
            .iter()
            .map(|obj| obj.borrow())
            .collect()
    }

    pub fn populate_if_needed(
        &mut self,
        on: Traversable,
        map: &Map,
        sim: &Sim,
        cs: &ColorScheme,
        prerender: &Prerender,
    ) {
        let now = sim.time();
        if Some(now) == self.time && self.agents_per_on.contains_key(&on) {
            return;
        }
        let step_count = sim.step_count();

        let mut list: Vec<Box<dyn GameRenderable>> = Vec::new();
        for c in sim.get_draw_cars(on, map).into_iter() {
            list.push(draw_vehicle(c, map, sim, prerender, cs));
        }
        let (loners, crowds) = sim.get_draw_peds(on, map);
        for p in loners {
            list.push(Box::new(DrawPedestrian::new(
                p, step_count, map, sim, prerender, cs,
            )));
        }
        for c in crowds {
            list.push(Box::new(DrawPedCrowd::new(c, map, prerender, cs)));
        }

        if Some(now) != self.time {
            self.agents_per_on.clear();
            self.time = Some(now);
        }

        self.agents_per_on.insert(on, list);
    }

    /// Recalculate the quadtree and drawable when required, limiting simulation-time refreshes to
    /// once every two real seconds. Initial, forced, and filter-change refreshes are immediate.
    pub fn calculate_unzoomed_agents<P: AsRef<Prerender>>(
        &mut self,
        prerender: &mut P,
        map: &Map,
        sim: &Sim,
        cs: &ColorScheme,
        force_refresh: bool,
    ) -> &QuadTree<AgentID> {
        let now = sim.time();
        let (sim_time_changed, filters_changed) = self
            .unzoomed
            .as_ref()
            .map(|(time, orig_agents, _, _)| (now != *time, self.unzoomed_agents != *orig_agents))
            .unwrap_or((false, false));
        let recalc = should_recalculate_unzoomed_agents(
            self.unzoomed.is_none(),
            sim_time_changed,
            filters_changed,
            force_refresh,
            self.unzoomed_last_refresh
                .as_ref()
                .map(|last_refresh| last_refresh.elapsed()),
        );

        if recalc {
            let highlighted = sim.get_highlighted_people();

            let mut batch = GeomBatch::new();
            let mut quadtree = QuadTree::builder();
            // It's quite silly to produce triangles for the same circle over and over again. ;)
            let car_circle = Circle::new(
                Pt2D::new(0.0, 0.0),
                unzoomed_agent_radius(Some(VehicleType::Car)),
            )
            .to_polygon();
            let ped_circle =
                Circle::new(Pt2D::new(0.0, 0.0), unzoomed_agent_radius(None)).to_polygon();

            for agent in sim.get_unzoomed_agents(map) {
                if let Some(mut color) = self.unzoomed_agents.color(&agent, cs) {
                    // If the sim has highlighted people, then fade all others out.
                    if highlighted
                        .as_ref()
                        .and_then(|h| agent.person.as_ref().map(|p| !h.contains(p)))
                        .unwrap_or(false)
                    {
                        // TODO Tune. How's this look at night?
                        color = color.tint(0.5);
                    }

                    let circle = if agent.id.to_vehicle_type().is_some() {
                        car_circle.translate(agent.pos.x(), agent.pos.y())
                    } else {
                        ped_circle.translate(agent.pos.x(), agent.pos.y())
                    };
                    quadtree.add_with_box(agent.id, circle.get_bounds());
                    batch.push(color, circle);
                }
            }

            let draw = prerender.as_ref().upload(batch);

            self.unzoomed = Some((now, self.unzoomed_agents.clone(), quadtree.build(), draw));
            self.unzoomed_last_refresh = Some(Instant::now());
        }

        &self.unzoomed.as_ref().unwrap().2
    }

    pub fn refresh_unzoomed_agents_next_time(&mut self) {
        self.unzoomed_last_refresh = None;
    }

    pub fn draw_unzoomed_agents(
        &mut self,
        g: &mut GfxCtx,
        map: &Map,
        sim: &Sim,
        cs: &ColorScheme,
        opts: &Options,
    ) {
        let active_drag = g.canvas.is_actively_dragging();
        if should_refresh_unzoomed_agents(active_drag, self.unzoomed.is_some()) {
            let force_refresh = g.canvas.is_dragging() && !active_drag;
            self.calculate_unzoomed_agents(g, map, sim, cs, force_refresh);
        }
        g.redraw(&self.unzoomed.as_ref().unwrap().3);

        if opts.debug_all_agents {
            let mut cnt = 0;
            for input in sim.get_all_draw_cars(map) {
                cnt += 1;
                draw_vehicle(input, map, sim, g.prerender, cs);
            }
            println!(
                "At {}, debugged {} cars",
                sim.time(),
                abstutil::prettyprint_usize(cnt)
            );
            // Pedestrians aren't the ones crashing
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct UnzoomedAgents {
    cars: bool,
    bikes: bool,
    buses_and_trains: bool,
    peds: bool,
}

impl UnzoomedAgents {
    pub fn new() -> UnzoomedAgents {
        UnzoomedAgents {
            cars: true,
            bikes: true,
            buses_and_trains: true,
            peds: true,
        }
    }

    pub fn cars(&self) -> bool {
        self.cars
    }
    pub fn bikes(&self) -> bool {
        self.bikes
    }
    pub fn buses_and_trains(&self) -> bool {
        self.buses_and_trains
    }
    pub fn peds(&self) -> bool {
        self.peds
    }

    fn color(&self, agent: &UnzoomedAgent, color_scheme: &ColorScheme) -> Option<Color> {
        match agent.id.to_vehicle_type() {
            Some(VehicleType::Car) => {
                if self.cars {
                    Some(color_scheme.unzoomed_car)
                } else {
                    None
                }
            }
            Some(VehicleType::Bike) => {
                if self.bikes {
                    Some(color_scheme.unzoomed_bike)
                } else {
                    None
                }
            }
            Some(VehicleType::Bus) | Some(VehicleType::Train) => {
                if self.buses_and_trains {
                    Some(color_scheme.unzoomed_bus)
                } else {
                    None
                }
            }
            None => {
                if self.peds {
                    Some(color_scheme.unzoomed_pedestrian)
                } else {
                    None
                }
            }
        }
    }

    pub fn update(&mut self, panel: &Panel) {
        self.cars = panel.is_checked("Car");
        self.bikes = panel.is_checked("Bike");
        self.buses_and_trains = panel.is_checked("Bus");
        self.peds = panel.is_checked("Walk");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use instant::Instant;

    use super::{should_recalculate_unzoomed_agents, should_refresh_unzoomed_agents, AgentCache};

    #[test]
    fn reuses_cached_unzoomed_agents_while_dragging() {
        assert!(!should_refresh_unzoomed_agents(true, true));
    }

    #[test]
    fn calculates_unzoomed_agents_when_drag_starts_without_a_cache() {
        assert!(should_refresh_unzoomed_agents(true, false));
    }

    #[test]
    fn refreshes_unzoomed_agents_after_dragging() {
        assert!(should_refresh_unzoomed_agents(false, true));
    }

    #[test]
    fn throttles_sim_time_refreshes_before_two_seconds() {
        assert!(!should_recalculate_unzoomed_agents(
            false,
            true,
            false,
            false,
            Some(Duration::from_millis(1_999)),
        ));
    }

    #[test]
    fn refreshes_sim_time_after_two_seconds() {
        assert!(should_recalculate_unzoomed_agents(
            false,
            true,
            false,
            false,
            Some(Duration::from_secs(2)),
        ));
    }

    #[test]
    fn forced_refresh_bypasses_the_interval() {
        assert!(should_recalculate_unzoomed_agents(
            false,
            true,
            false,
            true,
            Some(Duration::ZERO),
        ));
    }

    #[test]
    fn filter_changes_bypass_the_interval() {
        assert!(should_recalculate_unzoomed_agents(
            false,
            false,
            true,
            false,
            Some(Duration::ZERO),
        ));
    }

    #[test]
    fn missing_cache_bypasses_the_interval() {
        assert!(should_recalculate_unzoomed_agents(
            true, false, false, false, None,
        ));
    }

    #[test]
    fn invalidating_cache_forces_the_next_stale_refresh() {
        let mut cache = AgentCache::new();
        cache.unzoomed_last_refresh = Some(Instant::now());

        cache.refresh_unzoomed_agents_next_time();

        assert!(cache.unzoomed_last_refresh.is_none());
    }
}
