use widgetry::tools::{open_browser, PopupMsg, URLManager};
use widgetry::{EventCtx, Image, Line, Panel, SimpleState, State, Transition, Widget};

use crate::AppLike;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuSection {
    Primary,
    Secondary,
    More,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    ExploreCity,
    OpenProject,
    BeginnerHelp,
    SimulationChallenges,
    CommunityProposals,
    Ungap,
    FifteenMinuteNeighbourhoods,
    LowTrafficNeighbourhoods,
    ActDev,
    AdvancedTools,
    About,
}

struct MenuEntry {
    label: &'static str,
    tooltip: &'static str,
    action: MenuAction,
    section: MenuSection,
}

const MAIN_MENU_ENTRIES: &[MenuEntry] = &[
    MenuEntry {
        label: "Explore a city",
        tooltip: "Choose a real city, observe traffic, and test street changes",
        action: MenuAction::ExploreCity,
        section: MenuSection::Primary,
    },
    MenuEntry {
        label: "Open a project",
        tooltip: "Open an existing street-change proposal",
        action: MenuAction::OpenProject,
        section: MenuSection::Secondary,
    },
    MenuEntry {
        label: "New here?",
        tooltip: "Learn the basic A/B Street workflow",
        action: MenuAction::BeginnerHelp,
        section: MenuSection::Secondary,
    },
    MenuEntry {
        label: "Traffic simulation challenges",
        tooltip: "Complete specific objectives in the traffic simulator",
        action: MenuAction::SimulationChallenges,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "Community proposals",
        tooltip: "Try proposals for changing different cities",
        action: MenuAction::CommunityProposals,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "Ungap the Map",
        tooltip: "Improve a city's bike network",
        action: MenuAction::Ungap,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "15-minute neighbourhoods",
        tooltip: "Explore what places residents can easily reach",
        action: MenuAction::FifteenMinuteNeighbourhoods,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "Low traffic neighbourhoods",
        tooltip: "Reduce vehicle shortcuts through residential streets",
        action: MenuAction::LowTrafficNeighbourhoods,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "ActDev",
        tooltip: "Explore mobility patterns around new residential development",
        action: MenuAction::ActDev,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "Advanced tools",
        tooltip: "Open specialist and developer tools",
        action: MenuAction::AdvancedTools,
        section: MenuSection::More,
    },
    MenuEntry {
        label: "About",
        tooltip: "About A/B Street and its simulation assumptions",
        action: MenuAction::About,
        section: MenuSection::More,
    },
];

fn main_menu_entries() -> &'static [MenuEntry] {
    MAIN_MENU_ENTRIES
}

fn menu_entry(action: MenuAction) -> &'static MenuEntry {
    main_menu_entries()
        .iter()
        .find(|entry| entry.action == action)
        .unwrap()
}

fn more_tool_rows(window_width: f64) -> Vec<Vec<MenuAction>> {
    let actions_per_row = if window_width >= 1_200.0 { 4 } else { 2 };
    main_menu_entries()
        .iter()
        .filter(|entry| entry.section == MenuSection::More)
        .map(|entry| entry.action)
        .collect::<Vec<_>>()
        .chunks(actions_per_row)
        .map(|chunk| chunk.to_vec())
        .collect()
}

fn menu_button(ctx: &EventCtx, action: MenuAction, primary: bool) -> Widget {
    let entry = menu_entry(action);
    if primary {
        ctx.style()
            .btn_solid_primary
            .text(entry.label)
            .tooltip(entry.tooltip)
            .build_def(ctx)
    } else {
        ctx.style()
            .btn_outline
            .text(entry.label)
            .tooltip(entry.tooltip)
            .build_def(ctx)
    }
}

/// A title screen shared among all of the A/B Street apps.
pub struct TitleScreen<A: AppLike + 'static> {
    current_exe: Executable,
    enter_state: Box<dyn Fn(&mut EventCtx, &mut A, Vec<&str>) -> Box<dyn State<A>>>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Executable {
    ABStreet,
    FifteenMin,
    OSMViewer,
    ParkingMapper,
    RawMapEditor,
    LTN,
}

impl<A: AppLike + 'static> TitleScreen<A> {
    pub fn new_state(
        ctx: &mut EventCtx,
        _app: &A,
        current_exe: Executable,
        enter_state: Box<dyn Fn(&mut EventCtx, &mut A, Vec<&str>) -> Box<dyn State<A>>>,
    ) -> Box<dyn State<A>> {
        let mut more_tools = vec![Line("More tools").small_heading().into_widget(ctx)];
        for (idx, row) in more_tool_rows(ctx.canvas.window_width)
            .into_iter()
            .enumerate()
        {
            let row = Widget::row(
                row.into_iter()
                    .map(|action| menu_button(ctx, action, false))
                    .collect(),
            );
            more_tools.push(if idx == 0 {
                row.margin_below(10)
            } else {
                row.centered_horiz().margin_below(10)
            });
        }

        let panel = Panel::new_builder(Widget::col(vec![
            Widget::row(vec![
                Image::from_path("system/assets/pregame/logo.svg")
                    .untinted()
                    .dims(120.0)
                    .into_widget(ctx),
                Widget::col(vec![
                    Line("A/B Street").big_heading_plain().into_widget(ctx),
                    Line("Traffic simulation and street planning")
                        .small_heading()
                        .into_widget(ctx),
                ])
                .centered_vert(),
            ])
            .centered_horiz(),
            Widget::row(vec![
                Widget::col(vec![
                    Line("Explore a real city, understand its streets, then test your ideas.")
                        .into_widget(ctx),
                    menu_button(ctx, MenuAction::ExploreCity, true).margin_above(20),
                ])
                .section(ctx),
                Widget::col(vec![
                    menu_button(ctx, MenuAction::OpenProject, false).margin_below(10),
                    menu_button(ctx, MenuAction::BeginnerHelp, false),
                ])
                .section(ctx),
            ])
            .centered_horiz(),
            Widget::col(more_tools).section(ctx),
            Widget::col(vec![
                ctx.style()
                    .btn_plain
                    .text("Created by Dustin Carlino, Yuwen Li, & Michael Kirk")
                    .build_widget(ctx, "Credits"),
                built_info::maybe_update(ctx),
            ])
            .centered_horiz()
            .align_bottom(),
        ]))
        .build(ctx);
        <dyn SimpleState<_>>::new_state(
            panel,
            Box::new(TitleScreen {
                current_exe,
                enter_state,
            }),
        )
    }

    fn run(
        &self,
        ctx: &mut EventCtx,
        app: &mut A,
        exe: Executable,
        args: Vec<&str>,
    ) -> Transition<A> {
        if exe == self.current_exe {
            Transition::Push((self.enter_state)(ctx, app, args))
        } else {
            exe.replace_process(ctx, app, args);
            // On most platforms, this is unreachable. But on Windows, just keep the current app
            // open.
            Transition::Keep
        }
    }
}

impl Executable {
    /// Run the given executable with some arguments. On Mac and Linux, this replaces the current
    /// process. On Windows, this launches a new child process and leaves the current alone. On
    /// web, this makes the browser go to a new page.
    pub fn replace_process<A: AppLike + 'static>(
        self,
        ctx: &mut EventCtx,
        app: &A,
        args: Vec<&str>,
    ) -> Transition<A> {
        let mut args: Vec<String> = args.into_iter().map(|a| a.to_string()).collect();
        // Usually pass in the current map's path
        match self {
            Executable::RawMapEditor => {
                args.push(abstio::path_raw_map(app.map().get_name()));
                args.push(format!(
                    "--cam={}",
                    URLManager::get_cam_param(ctx, app.map().get_gps_bounds())
                ));
            }
            _ => {
                args.push(app.map().get_name().path());
            }
        }

        // On native, end the current process and start another.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::process::Command;

            // TODO find_exe panics; should return error instead
            let binary = crate::tools::find_exe(match self {
                Executable::ABStreet => "game",
                Executable::FifteenMin => "fifteen_min",
                Executable::OSMViewer => "osm_viewer",
                Executable::ParkingMapper => "parking_mapper",
                Executable::RawMapEditor => "map_editor",
                Executable::LTN => "ltn",
            });

            // We can only replace the current process on Linux/Mac
            #[cfg(not(windows))]
            {
                use std::os::unix::process::CommandExt;
                let err = Command::new(binary).args(args).exec();
                // We only get here if something broke
                Transition::Push(PopupMsg::new_state(ctx, "Error", vec![err.to_string()]))
            }

            // On Windows, all we can do is open a new child process. Not sure how to end the
            // current or detach.
            #[cfg(windows)]
            {
                abstutil::must_run_cmd(Command::new(binary).args(args));
                Transition::Keep
            }
        }

        // On web, leave the current page and go to another.
        #[cfg(target_arch = "wasm32")]
        {
            fn set_href(url: &str) -> anyhow::Result<()> {
                let window = web_sys::window().ok_or(anyhow!("no window?"))?;
                window.location().set_href(url).map_err(|err| {
                    anyhow!(err
                        .as_string()
                        .unwrap_or("window.location.set_href failed".to_string()))
                })
            }

            let page = match self {
                Executable::ABStreet => "abstreet",
                Executable::FifteenMin => "fifteen_min",
                Executable::OSMViewer => "osm_viewer",
                // This only works on native
                Executable::ParkingMapper => unreachable!(),
                Executable::RawMapEditor => "map_editor",
                Executable::LTN => "ltn",
            };
            let url = format!("{}.html{}", page, abstutil::args_to_query_string(args));
            if let Err(err) = set_href(&url) {
                return Transition::Push(PopupMsg::new_state(
                    ctx,
                    "Error",
                    vec![format!("Couldn't redirect to {}: {}", url, err)],
                ));
            }
            Transition::Keep
        }
    }
}

impl<A: AppLike + 'static> SimpleState<A> for TitleScreen<A> {
    fn on_click(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut A,
        x: &str,
        _: &mut Panel,
    ) -> Transition<A> {
        let action = main_menu_entries()
            .iter()
            .find(|entry| entry.label == x)
            .map(|entry| entry.action);

        match action {
            Some(MenuAction::ExploreCity) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--explore"])
            }
            Some(MenuAction::OpenProject) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--open-project"])
            }
            Some(MenuAction::BeginnerHelp) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--starter"])
            }
            Some(MenuAction::SimulationChallenges) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--challenges"])
            }
            Some(MenuAction::CommunityProposals) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--proposals"])
            }
            Some(MenuAction::Ungap) => self.run(ctx, app, Executable::ABStreet, vec!["--ungap"]),
            Some(MenuAction::FifteenMinuteNeighbourhoods) => {
                self.run(ctx, app, Executable::FifteenMin, vec![])
            }
            Some(MenuAction::LowTrafficNeighbourhoods) => {
                self.run(ctx, app, Executable::LTN, vec![])
            }
            Some(MenuAction::ActDev) => {
                open_browser("https://actdev.cyipt.bike");
                Transition::Keep
            }
            Some(MenuAction::AdvancedTools) => {
                self.run(ctx, app, Executable::ABStreet, vec!["--devtools"])
            }
            Some(MenuAction::About) => Transition::Push(PopupMsg::new_state(
                ctx,
                "About A/B Street",
                vec![
                    "A/B Street helps you explore traffic and test possible street changes.",
                    "Results are estimates based on available map data and a simplified",
                    "simulation model. Use them to explore ideas and support discussion,",
                    "not as a final engineering or policy decision.",
                ],
            )),
            None if x == "Credits" => {
                open_browser("https://a-b-street.github.io/docs/project/team.html");
                Transition::Keep
            }
            None if x == "Download the new release" => {
                open_browser("https://github.com/a-b-street/abstreet/releases");
                Transition::Keep
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused, clippy::logic_bug)]
mod built_info {
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/built.rs"));

    pub fn maybe_update(ctx: &mut EventCtx) -> Widget {
        let t = built::util::strptime(BUILT_TIME_UTC);

        let txt = widgetry::Text::from(format!("This version built on {}", t.date_naive()))
            .into_widget(ctx);
        // Disable this warning; no promise about a release schedule anymore
        if false && (chrono::Utc::now() - t).num_days() > 15 {
            Widget::row(vec![
                txt.centered_vert(),
                ctx.style()
                    .btn_outline
                    .text("Download the new release")
                    .build_def(ctx),
            ])
        } else {
            txt
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_menu_has_one_map_first_primary_action() {
        let entries = main_menu_entries();
        let primary = entries
            .iter()
            .filter(|entry| entry.section == MenuSection::Primary)
            .collect::<Vec<_>>();

        assert_eq!(primary.len(), 1);
        assert_eq!(primary[0].action, MenuAction::ExploreCity);
        assert_eq!(primary[0].label, "Explore a city");
    }

    #[test]
    fn focused_menu_does_not_expose_santa() {
        assert!(main_menu_entries()
            .iter()
            .all(|entry| !entry.label.to_ascii_lowercase().contains("santa")));
    }

    #[test]
    fn specialist_features_live_under_more_tools() {
        for action in [
            MenuAction::Ungap,
            MenuAction::FifteenMinuteNeighbourhoods,
            MenuAction::LowTrafficNeighbourhoods,
            MenuAction::AdvancedTools,
        ] {
            let entry = main_menu_entries()
                .iter()
                .find(|entry| entry.action == action)
                .unwrap();
            assert_eq!(entry.section, MenuSection::More);
        }
    }

    #[test]
    fn more_tools_form_two_compact_rows() {
        let rows = more_tool_rows(1_920.0);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 4);
        assert_eq!(rows[1].len(), 4);

        let narrow_rows = more_tool_rows(900.0);
        assert_eq!(narrow_rows.len(), 4);
        assert!(narrow_rows.iter().all(|row| row.len() == 2));
    }
}

#[cfg(target_arch = "wasm32")]
mod built_info {
    use super::*;

    pub fn maybe_update(_: &mut EventCtx) -> Widget {
        Widget::nothing()
    }
}
