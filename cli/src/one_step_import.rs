use anyhow::{bail, Result};

use abstio::{CityName, MapName};
use geom::LonLat;

pub(crate) fn target_map_name(country: &str, city: &str, map: &str) -> Result<MapName> {
    for (label, component) in [("city", city), ("map", map)] {
        if component.is_empty()
            || component.contains(' ')
            || component.contains('/')
            || component.contains('\\')
        {
            bail!(
                "{} must be a non-empty, filename-safe name: {}",
                label,
                component
            );
        }
    }
    let city = CityName::parse(&format!("{}/{}", country, city))?;
    Ok(MapName::from_city(&city, map))
}

#[cfg(test)]
mod tests {
    use super::target_map_name;
    use abstio::MapName;

    #[test]
    fn defaults_keep_one_shot_imports_in_the_fake_city() {
        assert_eq!(
            target_map_name("zz", "oneshot", "demo").unwrap(),
            MapName::new("zz", "oneshot", "demo")
        );
    }

    #[test]
    fn named_imports_can_target_a_real_city() {
        assert_eq!(
            target_map_name("my", "kuching", "center").unwrap(),
            MapName::new("my", "kuching", "center")
        );
    }

    #[test]
    fn target_components_must_be_filename_safe() {
        assert!(target_map_name("my", "", "center").is_err());
        assert!(target_map_name("my", "kuching", "city center").is_err());
    }
}

pub async fn run(
    geojson_path: String,
    country: String,
    city: String,
    map: String,
    use_geofabrik: bool,
    use_osmium: bool,
    options: convert_osm::Options,
    create_uk_travel_demand_model: bool,
    opts: map_model::RawToMapOptions,
) -> Result<()> {
    let name = target_map_name(&country, &city, &map)?;
    let city = name.city.clone();
    let osm;
    if !use_geofabrik {
        println!("Downloading OSM data from Overpass...");
        osm = city.input_path(format!("osm/{}.osm", name.map));

        let geojson = abstio::slurp_file(geojson_path.clone())?;
        let mut polygons = LonLat::parse_geojson_polygons(String::from_utf8(geojson)?)?;
        let mut filter = "poly:\"".to_string();
        for pt in polygons.pop().unwrap().0 {
            filter.push_str(&format!("{} {} ", pt.y(), pt.x()));
        }
        filter.pop();
        filter.push('"');
        // See https://wiki.openstreetmap.org/wiki/Overpass_API/Overpass_QL
        let query = format!(
            "(\n   nwr({});\n     node(w)->.x;\n   <;\n);\nout meta;\n",
            filter
        );
        abstio::download_to_file("https://overpass-api.de/api/interpreter", Some(query), &osm)
            .await?;
    } else {
        println!("Figuring out what Geofabrik file contains your boundary");
        let (url, pbf) = importer::pick_geofabrik(geojson_path.clone()).await?;
        osm = city.input_path(format!("osm/{}.osm.pbf", name.map));
        fs_err::create_dir_all(std::path::Path::new(&pbf).parent().unwrap())
            .expect("Creating parent dir failed");
        fs_err::create_dir_all(std::path::Path::new(&osm).parent().unwrap())
            .expect("Creating parent dir failed");

        // Download it!
        // TODO This is timing out. Also, really could use progress bars.
        if !abstio::file_exists(&pbf) {
            println!("Downloading {}", url);
            abstio::download_to_file(url, None, &pbf).await?;
        }

        // Clip it
        println!("Clipping {pbf} to your boundary");
        if use_osmium {
            importer::osmium(
                pbf,
                geojson_path.clone(),
                osm.clone(),
                &importer::ImporterConfiguration::load(),
            );
        } else {
            crate::clip_osm::run(pbf, geojson_path.clone(), osm.clone())?;
        }
    }

    // Import!
    println!("Running importer");
    importer::oneshot(
        name,
        osm,
        Some(geojson_path),
        options,
        create_uk_travel_demand_model,
        opts,
    )
    .await;

    Ok(())
}
