use widgetry::{EventCtx, State};

use crate::app::{App, Transition};
use crate::challenges::ChallengesPicker;
use crate::sandbox::gameplay::Tutorial;
use crate::sandbox::SandboxMode;

pub(crate) use self::starter::StarterPanel;
use self::starter::{default_gameplay_mode, OpenProject};

pub mod proposals;
mod starter;

pub struct TitleScreen;

impl TitleScreen {
    pub fn new_state(ctx: &mut EventCtx, app: &mut App) -> Box<dyn State<App>> {
        map_gui::tools::TitleScreen::new_state(
            ctx,
            app,
            map_gui::tools::Executable::ABStreet,
            Box::new(enter_state),
        )
    }
}

pub(crate) fn enter_state(
    ctx: &mut EventCtx,
    app: &mut App,
    args: Vec<&str>,
) -> Box<dyn State<App>> {
    match args[0] {
        "--tutorial-intro" => Tutorial::start(ctx, app),
        "--challenges" => ChallengesPicker::new_state(ctx, app),
        "--explore" => map_gui::tools::CityPicker::new_state(
            ctx,
            app,
            Box::new(|ctx, _| Transition::Replace(StarterPanel::new_state(ctx))),
        ),
        "--open-project" => OpenProject::new_state(),
        "--starter" => StarterPanel::new_state(ctx),
        "--sandbox" => SandboxMode::simple_new(app, default_gameplay_mode(app)),
        "--proposals" => proposals::Proposals::new_state(ctx, None),
        "--ungap" => {
            let layers = crate::ungap::Layers::new(ctx, app);
            crate::ungap::ExploreMap::new_state(ctx, app, layers)
        }
        "--devtools" => crate::devtools::DevToolsMode::new_state(ctx, app),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_actions_use_plain_language() {
        let labels = starter::StarterAction::all()
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "Observe traffic",
                "Test a street change",
                "Compare before and after",
            ]
        );
    }
}
