use abstio::MapName;
use abstutil::Timer;
use geom::{Duration, Time};
use map_model::{DrivingSide, Map};
use sim::{AlertHandler, ScenarioGenerator, Sim, SimFlags, SimOptions};

#[test]
#[ignore = "requires locally generated Sarawak map binaries"]
fn compact_sarawak_maps_run_synthetic_traffic() {
    for city in ["bintulu", "kuching", "miri", "sibu"] {
        let name = MapName::new("my", city, "center");
        let mut timer = Timer::new(format!("validate {}", city));
        let map = Map::load_synchronously(name.path(), &mut timer);

        assert_eq!(map.get_config().driving_side, DrivingSide::Left);
        assert!(map.all_roads().len() > 100, "{city} has too few roads");
        assert!(
            map.all_intersections().len() > 100,
            "{city} has too few intersections"
        );
        assert!(
            map.all_buildings().len() > 50,
            "{city} has too few buildings"
        );
        assert!(
            !map.all_incoming_borders().is_empty() && !map.all_outgoing_borders().is_empty(),
            "{city} needs usable map borders"
        );

        let mut rng = SimFlags::for_test("sarawak_map_validation").make_rng();
        let scenario = ScenarioGenerator::small_run(&map).generate(&map, &mut rng, &mut timer);
        assert!(!scenario.people.is_empty(), "{city} generated no trips");

        let mut options = SimOptions::new("sarawak_map_validation");
        options.alerts = AlertHandler::Silence;
        let mut simulation = Sim::new(&map, options);
        let mut rng = SimFlags::for_test("sarawak_map_validation").make_rng();
        simulation.instantiate(&scenario, &map, &mut rng, &mut timer);
        simulation.timed_step(
            &map,
            Duration::minutes(30),
            &mut None,
            &mut Timer::throwaway(),
        );
        assert!(simulation.time() >= Time::START_OF_DAY + Duration::minutes(30));
    }
}
