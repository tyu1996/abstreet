use std::collections::BTreeMap;

use abstio::{CityName, Manifest, MapName};
use geom::{Distance, Percent};
use map_model::City;
use widgetry::tools::FileLoader;
use widgetry::{
    lctrl, Autocomplete, ClickOutcome, ControlState, DrawBaselayer, DrawWithTooltips, EventCtx,
    GeomBatch, GfxCtx, Image, Key, Line, Outcome, Panel, PanelDims, RewriteColor, State, Text,
    TextExt, Transition, Widget,
};

use crate::load::MapLoader;
use crate::render::DrawArea;
use crate::tools::{grey_out_map, nice_country_name, nice_map_name};
use crate::AppLike;

/// Lets the player switch maps.
pub struct CityPicker<A: AppLike> {
    panel: Panel,
    // Wrapped in an Option just to make calling from event() work.
    on_load: Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
}

impl<A: AppLike + 'static> CityPicker<A> {
    pub fn new_state(
        ctx: &mut EventCtx,
        app: &A,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        BrowseCities::new_state(ctx, app, on_load, app.map().get_city_name().country.clone())
    }

    fn new_in_city(
        ctx: &mut EventCtx,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
        city_name: CityName,
    ) -> Box<dyn State<A>> {
        FileLoader::<A, City>::new_state(
            ctx,
            abstio::path(format!(
                "system/{}/{}/city.bin",
                city_name.country, city_name.city
            )),
            Box::new(move |ctx, app, _, maybe_city| {
                // If city.bin exists, use it to draw the district map.
                let district_picker = if let Ok(city) = maybe_city {
                    let bounds = city.boundary.get_bounds();

                    let zoom = (0.8 * ctx.canvas.window_width / bounds.width())
                        .min(0.8 * ctx.canvas.window_height / bounds.height());

                    let mut batch = GeomBatch::new();
                    batch.push(app.cs().map_background.clone(), city.boundary);
                    for (area_type, polygon) in city.areas {
                        batch.push(DrawArea::fill(area_type, app.cs()), polygon);
                    }

                    // If somebody has just generated a new map somewhere with an existing
                    // city.bin, but hasn't updated city.bin yet, that new map will be invisible on
                    // the city-wide diagram.
                    let outline_color = app.cs().minimap_cursor_border;
                    let mut tooltips = Vec::new();
                    for (name, polygon) in city.districts {
                        if &name != app.map().get_name() {
                            if let Ok(zoomed_polygon) = polygon.scale(zoom) {
                                batch.push(
                                    outline_color,
                                    polygon.to_outline(Distance::meters(200.0)),
                                );
                                tooltips.push((
                                    zoomed_polygon,
                                    Text::from(nice_map_name(&name)),
                                    Some(ClickOutcome::Custom(Box::new(name))),
                                ));
                            }
                        }
                    }
                    DrawWithTooltips::new_widget(
                        ctx,
                        batch.scale(zoom),
                        tooltips,
                        Box::new(move |poly| {
                            GeomBatch::from(vec![(outline_color.alpha(0.5), poly.clone())])
                        }),
                    )
                } else {
                    Widget::nothing()
                };

                // Use the filesystem to list the buttons on the side.
                // (There's no point in listing these from city.bin if it exists -- if somebody
                // imports a new map in an existing city, it could be out of sync anyway.)
                let mut this_city =
                    vec![format!("More districts in {}", city_name.describe()).text_widget(ctx)];
                for name in MapName::list_all_maps_in_city_merged(&city_name, &Manifest::load()) {
                    this_city.push(
                        ctx.style()
                            .btn_outline
                            .text(nice_map_name(&name))
                            .no_tooltip()
                            .disabled(&name == app.map().get_name())
                            .build_widget(ctx, &name.path()),
                    );
                }

                let mut other_places = vec![Line("Other places").into_widget(ctx)];
                for (country, cities) in cities_per_country() {
                    // If there's only one city and we're already there, skip it.
                    if cities.len() == 1 && cities[0] == city_name {
                        continue;
                    }
                    let flag_path = format!("system/assets/flags/{}.svg", country);
                    if abstio::file_exists(abstio::path(&flag_path)) {
                        other_places.push(
                            ctx.style()
                                .btn_outline
                                .icon_text(
                                    &flag_path,
                                    format!("{} in {}", cities.len(), nice_country_name(&country)),
                                )
                                .image_color(RewriteColor::NoOp, ControlState::Default)
                                .image_dims(30.0)
                                .build_widget(ctx, &country),
                        );
                    } else {
                        other_places.push(
                            ctx.style()
                                .btn_outline
                                .text(format!(
                                    "{} in {}",
                                    cities.len(),
                                    nice_country_name(&country)
                                ))
                                .build_widget(ctx, country),
                        );
                    }
                }

                Transition::Replace(Box::new(CityPicker {
                    on_load: Some(on_load),
                    panel: Panel::new_builder(Widget::col(vec![
                        Widget::row(vec![
                            Line("Select a district").small_heading().into_widget(ctx),
                            ctx.style().btn_close_widget(ctx),
                        ]),
                        if cfg!(target_arch = "wasm32") {
                            // On web, this is a link, so it's styled appropriately.
                            ctx.style()
                                .btn_plain
                                .btn()
                                .label_underlined_text("Import a new city into A/B Street")
                                .build_widget(ctx, "import new city")
                        } else {
                            // On native this shows the "import" instructions modal within
                            // the app
                            Widget::row(vec![
                                ctx.style()
                                    .btn_outline
                                    .text("Import a new city into A/B Street")
                                    .build_widget(ctx, "import new city"),
                                ctx.style()
                                    .btn_outline
                                    .text("Re-import this map with latest OpenStreetMap data")
                                    .tooltip("OSM edits take a few minutes to appear in Overpass. Note this will create a new copy of the map, not overwrite the original.")
                                    .build_widget(ctx, "re-import this city"),
                            ])
                        },
                        ctx.style()
                            .btn_outline
                            .icon_text("system/assets/tools/search.svg", "Search all maps")
                            .hotkey(lctrl(Key::F))
                            .build_def(ctx),
                        Widget::row(vec![
                            Widget::col(other_places).centered_vert(),
                            district_picker,
                            Widget::col(this_city).centered_vert(),
                        ]),
                    ]))
                    .build(ctx),
                }))
            }),
        )
    }
}

impl<A: AppLike + 'static> State<A> for CityPicker<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        // TODO This happens if we prompt the user to download something, but they cancel. At that
        // point, we've lost the callback, so for now, just totally bail out.
        if self.on_load.is_none() {
            return Transition::Pop;
        }

        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "close" => {
                    return Transition::Pop;
                }
                "Search all maps" => {
                    return Transition::Replace(AllCityPicker::new_state(
                        ctx,
                        self.on_load.take().unwrap(),
                    ));
                }
                "import new city" => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        widgetry::tools::open_browser(
                            "https://a-b-street.github.io/docs/user/new_city.html",
                        );
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        return Transition::Replace(crate::tools::importer::ImportCity::new_state(
                            ctx,
                            self.on_load.take().unwrap(),
                        ));
                    }
                }
                "re-import this city" => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        unreachable!()
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        return reimport_city(ctx, app);
                    }
                }
                x => {
                    if let Some(name) = MapName::from_path(x) {
                        return chose_city(ctx, app, name, &mut self.on_load);
                    }
                    // Browse cities for another country
                    return Transition::Replace(CitiesInCountryPicker::new_state(
                        ctx,
                        app,
                        self.on_load.take().unwrap(),
                        x,
                    ));
                }
            },
            Outcome::ClickCustom(data) => {
                let name = data.as_any().downcast_ref::<MapName>().unwrap();
                return chose_city(ctx, app, name.clone(), &mut self.on_load);
            }
            _ => {}
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

struct AllCityPicker<A: AppLike> {
    panel: Panel,
    // Wrapped in an Option just to make calling from event() work.
    on_load: Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
}

impl<A: AppLike + 'static> AllCityPicker<A> {
    fn new_state(
        ctx: &mut EventCtx,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        let mut autocomplete_entries = Vec::new();
        for name in MapName::list_all_maps_merged(&Manifest::load()) {
            autocomplete_entries.push((name.describe(), name.path()));
        }

        Box::new(AllCityPicker {
            on_load: Some(on_load),
            panel: Panel::new_builder(Widget::col(vec![
                Widget::row(vec![
                    Line("Select a district").small_heading().into_widget(ctx),
                    ctx.style().btn_close_widget(ctx),
                ]),
                Widget::row(vec![
                    Image::from_path("system/assets/tools/search.svg").into_widget(ctx),
                    Autocomplete::new_widget(ctx, autocomplete_entries, 10).named("search"),
                ])
                .padding(8),
            ]))
            .dims_width(PanelDims::ExactPercent(0.8))
            .dims_height(PanelDims::ExactPercent(0.8))
            .build(ctx),
        })
    }
}

impl<A: AppLike + 'static> State<A> for AllCityPicker<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        // Same as CityPicker
        if self.on_load.is_none() {
            return Transition::Pop;
        }

        if let Outcome::Clicked(x) = self.panel.event(ctx) {
            match x.as_ref() {
                "close" => {
                    return Transition::Pop;
                }
                _ => unreachable!(),
            }
        }
        if let Some(mut paths) = self.panel.autocomplete_done::<String>("search") {
            if !paths.is_empty() {
                return chose_city(
                    ctx,
                    app,
                    MapName::from_path(&paths.remove(0)).unwrap(),
                    &mut self.on_load,
                );
            }
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

struct CitiesInCountryPicker<A: AppLike> {
    panel: Panel,
    // Wrapped in an Option just to make calling from event() work.
    on_load: Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
}

impl<A: AppLike + 'static> CitiesInCountryPicker<A> {
    fn new_state(
        ctx: &mut EventCtx,
        app: &A,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
        country: &str,
    ) -> Box<dyn State<A>> {
        let flag_path = format!("system/assets/flags/{}.svg", country);
        let draw_flag = if abstio::file_exists(abstio::path(&flag_path)) {
            let flag = GeomBatch::load_svg(ctx, format!("system/assets/flags/{}.svg", country));
            let y_factor = 30.0 / flag.get_dims().height;
            flag.scale(y_factor).into_widget(ctx)
        } else {
            Widget::nothing()
        };
        let mut col = vec![Widget::row(vec![
            draw_flag,
            Line(format!("Select a city in {}", nice_country_name(country)))
                .small_heading()
                .into_widget(ctx),
            ctx.style().btn_close_widget(ctx),
        ])];

        let mut buttons = Vec::new();
        let mut last_letter = ' ';
        for city in cities_per_country().remove(country).unwrap() {
            if &city == app.map().get_city_name() {
                continue;
            }
            let letter = city
                .city
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .next()
                .unwrap();
            if last_letter != letter {
                if !buttons.is_empty() {
                    let mut row = vec![Line(last_letter)
                        .small_heading()
                        .into_widget(ctx)
                        .margin_right(20)];
                    row.append(&mut buttons);
                    col.push(
                        Widget::custom_row(row).flex_wrap_no_inner_spacing(ctx, Percent::int(70)),
                    );
                }

                last_letter = letter;
            }

            buttons.push(
                ctx.style()
                    .btn_outline
                    .text(&city.city)
                    .build_widget(ctx, &city.to_path())
                    .margin_right(10)
                    .margin_below(10),
            );
        }
        if !buttons.is_empty() {
            let mut row = vec![Line(last_letter)
                .small_heading()
                .into_widget(ctx)
                .margin_right(20)];
            row.append(&mut buttons);
            col.push(Widget::custom_row(row).flex_wrap_no_inner_spacing(ctx, Percent::int(70)));
        }

        Box::new(CitiesInCountryPicker {
            on_load: Some(on_load),
            panel: Panel::new_builder(Widget::col(col))
                .dims_width(PanelDims::ExactPercent(0.8))
                .dims_height(PanelDims::ExactPercent(0.8))
                .build(ctx),
        })
    }
}

impl<A: AppLike + 'static> State<A> for CitiesInCountryPicker<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        // Same as CityPicker
        if self.on_load.is_none() {
            return Transition::Pop;
        }

        if let Outcome::Clicked(x) = self.panel.event(ctx) {
            match x.as_ref() {
                "close" => {
                    // Go back to the screen that lets you choose all countries.
                    return Transition::Replace(CityPicker::new_state(
                        ctx,
                        app,
                        self.on_load.take().unwrap(),
                    ));
                }
                path => {
                    let city = CityName::parse(path).unwrap();
                    let mut maps = MapName::list_all_maps_in_city_merged(&city, &Manifest::load());
                    if maps.len() == 1 {
                        return chose_city(ctx, app, maps.pop().unwrap(), &mut self.on_load);
                    }

                    // We may need to grab city.bin
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let path = format!("system/{}/{}/city.bin", city.country, city.city);
                        if Manifest::load()
                            .entries
                            .contains_key(&format!("data/{}", path))
                            && !abstio::file_exists(abstio::path(path))
                        {
                            return crate::tools::prompt_to_download_missing_data(
                                ctx,
                                maps.pop().unwrap(),
                                self.on_load.take().unwrap(),
                            );
                        }
                    }

                    return Transition::Replace(CityPicker::new_in_city(
                        ctx,
                        self.on_load.take().unwrap(),
                        city,
                    ));
                }
            }
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

fn cities_per_country() -> BTreeMap<String, Vec<CityName>> {
    cities_per_country_from_manifest(&Manifest::load())
}

fn cities_per_country_from_manifest(manifest: &Manifest) -> BTreeMap<String, Vec<CityName>> {
    let mut per_country = BTreeMap::new();
    for city in CityName::list_all_cities_merged(manifest) {
        per_country
            .entry(city.country.clone())
            .or_insert_with(Vec::new)
            .push(city);
    }
    per_country
}

fn maps_per_city(manifest: &Manifest) -> BTreeMap<CityName, Vec<MapName>> {
    let mut per_city = BTreeMap::new();
    for map in MapName::list_all_maps_merged(manifest) {
        per_city
            .entry(map.city.clone())
            .or_insert_with(Vec::new)
            .push(map);
    }
    per_city
}

struct BrowseCities<A: AppLike> {
    panel: Panel,
    on_load: Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
}

struct CityBrowserLayout {
    autocomplete_chars: usize,
    search_results: usize,
    geography_width_percent: usize,
    panel_width_percent: f64,
    panel_height_percent: f64,
    stacked: bool,
}

fn city_browser_layout(window_width: f64) -> CityBrowserLayout {
    if window_width >= 1_100.0 {
        CityBrowserLayout {
            autocomplete_chars: 26,
            search_results: 6,
            geography_width_percent: 34,
            panel_width_percent: 0.82,
            panel_height_percent: 0.82,
            stacked: false,
        }
    } else {
        CityBrowserLayout {
            autocomplete_chars: 26,
            search_results: 6,
            geography_width_percent: 78,
            panel_width_percent: 0.9,
            panel_height_percent: 0.86,
            stacked: true,
        }
    }
}

impl<A: AppLike + 'static> BrowseCities<A> {
    fn new_state(
        ctx: &mut EventCtx,
        _app: &A,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
        selected_country: String,
    ) -> Box<dyn State<A>> {
        let layout = city_browser_layout(ctx.canvas.window_width);
        let manifest = Manifest::load();
        let per_country = cities_per_country_from_manifest(&manifest);
        let maps_per_city = maps_per_city(&manifest);
        let autocomplete_entries = per_country
            .values()
            .flatten()
            .map(|city| (city_search_label(city), city.to_path()))
            .collect::<Vec<_>>();

        let mut city_rows = vec![Line(format!(
            "Cities in {}",
            nice_country_name(&selected_country)
        ))
        .small_heading()
        .into_widget(ctx)];
        if let Some(cities) = per_country.get(&selected_country) {
            for city in cities {
                let maps = maps_per_city
                    .get(city)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                #[cfg(not(target_arch = "wasm32"))]
                let installed = maps
                    .iter()
                    .filter(|name| abstio::file_exists(name.path()))
                    .count();
                #[cfg(target_arch = "wasm32")]
                let installed = 0;
                city_rows.push(
                    Widget::row(vec![
                        ctx.style()
                            .btn_outline
                            .text(city.city.replace('_', " "))
                            .build_widget(ctx, format!("city:{}", city.to_path())),
                        Line(city_availability_label(
                            installed,
                            maps.len(),
                            cfg!(target_arch = "wasm32"),
                        ))
                        .into_widget(ctx)
                        .centered_vert(),
                    ])
                    .margin_below(8),
                );
            }
        }

        let mut by_region: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
        for country in per_country.keys() {
            by_region
                .entry(geographic_region(country))
                .or_default()
                .push(country.clone());
        }

        let mut region_columns = Vec::new();
        for region in [
            "Americas",
            "Europe",
            "Africa and Middle East",
            "Asia-Pacific",
            "Other",
        ] {
            let Some(countries) = by_region.remove(region) else {
                continue;
            };
            let mut rows = vec![Line(region).small_heading().into_widget(ctx)];
            for country in countries {
                let count = per_country[&country].len();
                let label = format!("{} ({})", nice_country_name(&country), count);
                let flag_path = format!("system/assets/flags/{}.svg", country);
                let button = if abstio::file_exists(abstio::path(&flag_path)) {
                    ctx.style()
                        .btn_outline
                        .icon_text(&flag_path, label)
                        .image_color(RewriteColor::NoOp, ControlState::Default)
                        .image_dims(24.0)
                        .disabled(country == selected_country)
                        .build_widget(ctx, format!("country:{}", country))
                } else {
                    ctx.style()
                        .btn_outline
                        .text(label)
                        .disabled(country == selected_country)
                        .build_widget(ctx, format!("country:{}", country))
                };
                rows.push(button.margin_below(6));
            }
            region_columns.push(Widget::col(rows).margin_right(12));
        }

        let city_list = Widget::col(vec![
            Widget::row(vec![
                Image::from_path("system/assets/tools/search.svg").into_widget(ctx),
                Autocomplete::new_compact_widget(
                    ctx,
                    autocomplete_entries,
                    layout.search_results,
                    layout.autocomplete_chars,
                )
                .named("city search"),
            ])
            .padding(8),
            Widget::col(city_rows),
        ])
        .section(ctx)
        .margin_right(15);
        let geographic_browser = Widget::col(vec![
            Line("Browse the world").small_heading().into_widget(ctx),
            Line("Choose a country to filter the city list.").into_widget(ctx),
            Widget::custom_row(region_columns)
                .flex_wrap(ctx, Percent::int(layout.geography_width_percent)),
        ])
        .section(ctx);
        let browser = if layout.stacked {
            Widget::col(vec![city_list, geographic_browser])
        } else {
            Widget::row(vec![city_list, geographic_browser])
        };

        Box::new(BrowseCities {
            on_load: Some(on_load),
            panel: Panel::new_builder(Widget::col(vec![
                Widget::row(vec![
                    Line("Choose a city").small_heading().into_widget(ctx),
                    ctx.style().btn_close_widget(ctx),
                ]),
                browser,
            ]))
            .dims_width(PanelDims::ExactPercent(layout.panel_width_percent))
            .dims_height(PanelDims::ExactPercent(layout.panel_height_percent))
            .build(ctx),
        })
    }
}

impl<A: AppLike + 'static> State<A> for BrowseCities<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        if self.on_load.is_none() {
            return Transition::Pop;
        }

        if let Outcome::Clicked(x) = self.panel.event(ctx) {
            if x == "close" {
                return Transition::Pop;
            }
            if let Some(country) = x.strip_prefix("country:") {
                return Transition::Replace(BrowseCities::new_state(
                    ctx,
                    app,
                    self.on_load.take().unwrap(),
                    country.to_string(),
                ));
            }
            if let Some(city) = x.strip_prefix("city:") {
                return chose_city_name(
                    ctx,
                    app,
                    CityName::parse(city).unwrap(),
                    &mut self.on_load,
                );
            }
        }

        if let Some(mut city_paths) = self.panel.autocomplete_done::<String>("city search") {
            if !city_paths.is_empty() {
                return chose_city_name(
                    ctx,
                    app,
                    CityName::parse(&city_paths.remove(0)).unwrap(),
                    &mut self.on_load,
                );
            }
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

fn chose_city_name<A: AppLike + 'static>(
    ctx: &mut EventCtx,
    app: &mut A,
    city: CityName,
    on_load: &mut Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
) -> Transition<A> {
    let mut maps = MapName::list_all_maps_in_city_merged(&city, &Manifest::load());
    if maps.len() == 1 {
        return chose_city(ctx, app, maps.pop().unwrap(), on_load);
    }

    Transition::Replace(CityPicker::new_in_city(ctx, on_load.take().unwrap(), city))
}

fn city_search_label(city: &CityName) -> String {
    format!(
        "{} ({})",
        city.city.replace('_', " "),
        nice_country_name(&city.country)
    )
}

fn city_availability_label(installed: usize, total: usize, online_only: bool) -> &'static str {
    if online_only {
        "Available online"
    } else if installed == 0 {
        "Download"
    } else if installed == total {
        "Installed"
    } else {
        "Partly installed"
    }
}

fn geographic_region(country: &str) -> &'static str {
    match country {
        "br" | "ca" | "cl" | "us" => "Americas",
        "at" | "ch" | "cz" | "de" | "fr" | "gb" | "nl" | "pl" | "pt" => "Europe",
        "il" | "ir" | "ly" => "Africa and Middle East",
        "au" | "hk" | "in" | "jp" | "kr" | "my" | "nz" | "sg" | "tw" => "Asia-Pacific",
        _ => "Other",
    }
}

fn chose_city<A: AppLike + 'static>(
    ctx: &mut EventCtx,
    app: &mut A,
    name: MapName,
    on_load: &mut Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
) -> Transition<A> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !abstio::file_exists(name.path()) {
            let on_load = on_load.take().unwrap();
            return crate::tools::prompt_to_download_missing_data(
                ctx,
                name.clone(),
                Box::new(move |ctx, app| {
                    Transition::Replace(MapLoader::new_state(ctx, app, name, on_load))
                }),
            );
        }
    }

    Transition::Replace(MapLoader::new_state(
        ctx,
        app,
        name,
        on_load.take().unwrap(),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn reimport_city<A: AppLike + 'static>(ctx: &mut EventCtx, app: &A) -> Transition<A> {
    let name = format!("updated_{}", app.map().get_name().as_filename());

    let args = vec![
        crate::tools::find_exe("cli"),
        "one-step-import".to_string(),
        "--geojson-path=boundary.json".to_string(),
        format!("--map-name={}", name),
    ];

    // Write the current map boundary
    abstio::write_json(
        "boundary.json".to_string(),
        &geom::geometries_to_geojson(vec![app
            .map()
            .get_boundary_polygon()
            .to_geojson(Some(app.map().get_gps_bounds()))]),
    );

    return Transition::Push(crate::tools::RunCommand::new_state(
        ctx,
        true,
        args,
        Box::new(|_, _, success, _| {
            if success {
                abstio::delete_file("boundary.json");

                Transition::ConsumeState(Box::new(move |state, ctx, app| {
                    let mut state = state.downcast::<CityPicker<A>>().ok().unwrap();
                    let on_load = state.on_load.take().unwrap();
                    let map_name = MapName::new("zz", "oneshot", &name);
                    vec![MapLoader::new_state(ctx, app, map_name, on_load)]
                }))
            } else {
                // The popup already explained the failure
                Transition::Keep
            }
        }),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_search_labels_include_city_and_country() {
        let seattle = CityName::new("us", "seattle");

        assert_eq!(
            city_search_label(&seattle),
            "seattle (United States of America)"
        );
    }

    #[test]
    fn countries_are_grouped_into_geographic_regions() {
        assert_eq!(geographic_region("us"), "Americas");
        assert_eq!(geographic_region("gb"), "Europe");
        assert_eq!(geographic_region("tw"), "Asia-Pacific");
        assert_eq!(geographic_region("my"), "Asia-Pacific");
        assert_eq!(geographic_region("ly"), "Africa and Middle East");
    }

    #[test]
    fn city_availability_distinguishes_partial_installations() {
        assert_eq!(city_availability_label(0, 3, false), "Download");
        assert_eq!(city_availability_label(1, 3, false), "Partly installed");
        assert_eq!(city_availability_label(3, 3, false), "Installed");
        assert_eq!(city_availability_label(0, 3, true), "Available online");
    }

    #[test]
    fn city_browser_layout_stays_compact_and_stacks_on_narrow_windows() {
        let wide = city_browser_layout(1_920.0);
        assert_eq!(wide.autocomplete_chars, 26);
        assert_eq!(wide.search_results, 6);
        assert_eq!(wide.geography_width_percent, 34);
        assert!(!wide.stacked);

        let narrow = city_browser_layout(900.0);
        assert_eq!(narrow.geography_width_percent, 78);
        assert!(narrow.stacked);
    }
}
