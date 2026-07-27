use abstio::MapName;
use widgetry::tools::PopupMsg;
use widgetry::tools::{FileLoader, RawBytes};
use widgetry::{EventCtx, GfxCtx, State, Transition};

use crate::AppLike;

const SARAWAK_PAGES_BASE: &str = "https://tyu1996.github.io/abstreet/sarawak-data-v1";

pub struct MapLoader;

fn remote_map_url(_version: &str, name: &MapName) -> Option<String> {
    if name.city.country == "my" {
        let path = format!(
            "data/system/{}/{}/maps/{}.bin",
            name.city.country, name.city.city, name.map
        );
        Some(format!(
            "{}/{}.gz",
            SARAWAK_PAGES_BASE,
            path.replace('/', "--")
        ))
    } else {
        None
    }
}

impl MapLoader {
    pub fn new_state<A: AppLike + 'static>(
        ctx: &mut EventCtx,
        app: &A,
        name: MapName,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        if app.map().get_name() == &name {
            return Box::new(MapAlreadyLoaded {
                on_load: Some(on_load),
            });
        }

        MapLoader::force_reload(ctx, name, on_load)
    }

    /// Even if the current map name matches, still reload.
    pub fn force_reload<A: AppLike + 'static>(
        ctx: &mut EventCtx,
        name: MapName,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        // TODO Generalize this more, maybe with some kind of country code -> font config
        if let Some(extra_font) = match name.city.country.as_ref() {
            "hk" | "jp" | "tw" => Some("NotoSerifCJKtc-Regular.otf"),
            "il" => Some("NotoSansHebrew-Regular.ttf"),
            "ir" | "ly" => Some("NotoSansArabic-Regular.ttf"),
            "kr" => Some("NotoSansKR-Regular.ttf"),
            _ => None,
        } {
            if !ctx.is_font_loaded(extra_font) {
                return FileLoader::<A, RawBytes>::new_state(
                    ctx,
                    abstio::path(format!("system/extra_fonts/{}", extra_font)),
                    Box::new(move |ctx, app, _, bytes| match bytes {
                        Ok(bytes) => {
                            ctx.load_font(extra_font, bytes.0);
                            Transition::Replace(MapLoader::new_state(ctx, app, name, on_load))
                        }
                        Err(err) => Transition::Replace(PopupMsg::new_state(
                            ctx,
                            "Error",
                            vec![format!("Couldn't load {}", extra_font), err.to_string()],
                        )),
                    }),
                );
            }
        }

        let path = if cfg!(target_arch = "wasm32") {
            remote_map_url(crate::tools::version(), &name).unwrap_or_else(|| name.path())
        } else {
            name.path()
        };
        FileLoader::<A, map_model::Map>::new_state(
            ctx,
            path,
            Box::new(move |ctx, app, timer, map| {
                match map {
                    Ok(mut map) => {
                        // Kind of a hack. We can't generically call Map::new with the FileLoader.
                        map.map_loaded_directly(timer);

                        app.map_switched(ctx, map, timer);

                        (on_load)(ctx, app)
                    }
                    Err(err) => Transition::Replace(PopupMsg::new_state(
                        ctx,
                        "Error",
                        vec![
                            format!("Couldn't load {}", name.describe()),
                            err.to_string(),
                        ],
                    )),
                }
            }),
        )
    }
}

struct MapAlreadyLoaded<A: AppLike> {
    on_load: Option<Box<dyn FnOnce(&mut EventCtx, &mut A) -> Transition<A>>>,
}

#[cfg(test)]
mod tests {
    use super::remote_map_url;
    use abstio::MapName;

    #[test]
    fn sarawak_web_maps_use_the_cors_enabled_owned_host() {
        assert!(remote_map_url("dev", &MapName::new("us", "seattle", "montlake")).is_none());
        assert_eq!(
            remote_map_url("dev", &MapName::new("my", "kuching", "center")).unwrap(),
            concat!(
                "https://tyu1996.github.io/abstreet/sarawak-data-v1/",
                "data--system--my--kuching--maps--center.bin.gz"
            )
        );
    }
}
impl<A: AppLike + 'static> State<A> for MapAlreadyLoaded<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        (self.on_load.take().unwrap())(ctx, app)
    }
    fn draw(&self, _: &mut GfxCtx, _: &A) {}
}
