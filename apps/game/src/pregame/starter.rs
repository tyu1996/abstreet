use synthpop::Scenario;
use widgetry::tools::PopupMsg;
use widgetry::{
    DrawBaselayer, EventCtx, GfxCtx, Line, Panel, SimpleState, State, Transition, Widget,
};

use crate::app::App;
use crate::edit::LoadEdits;
use crate::sandbox::{GameplayMode, SandboxMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StarterAction {
    ObserveTraffic,
    TestStreetChange,
    CompareBeforeAfter,
}

impl StarterAction {
    const ACTIONS: [StarterAction; 3] = [
        StarterAction::ObserveTraffic,
        StarterAction::TestStreetChange,
        StarterAction::CompareBeforeAfter,
    ];

    pub(super) fn all() -> &'static [StarterAction] {
        &Self::ACTIONS
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            StarterAction::ObserveTraffic => "Observe traffic",
            StarterAction::TestStreetChange => "Test a street change",
            StarterAction::CompareBeforeAfter => "Compare before and after",
        }
    }

    fn description(self) -> &'static str {
        match self {
            StarterAction::ObserveTraffic => {
                "Run the current scenario and inspect how people move."
            }
            StarterAction::TestStreetChange => {
                "Edit a street, then run the simulation to observe the effects."
            }
            StarterAction::CompareBeforeAfter => {
                "Save a proposal and use map layers and dashboards to compare results."
            }
        }
    }
}

#[derive(Clone, Copy)]
enum StarterContext {
    BeforeSandbox,
    InSandbox,
}

pub(crate) struct StarterPanel {
    context: StarterContext,
}

impl StarterPanel {
    pub(crate) fn new_state(ctx: &mut EventCtx) -> Box<dyn State<App>> {
        Self::build(ctx, StarterContext::BeforeSandbox)
    }

    pub(crate) fn help_state(ctx: &mut EventCtx) -> Box<dyn State<App>> {
        Self::build(ctx, StarterContext::InSandbox)
    }

    fn build(ctx: &mut EventCtx, context: StarterContext) -> Box<dyn State<App>> {
        let mut rows = vec![
            Widget::row(vec![
                Line("What would you like to do?")
                    .small_heading()
                    .into_widget(ctx),
                ctx.style().btn_close_widget(ctx),
            ]),
            Line("Choose a starting task, or close this panel to explore freely.")
                .into_widget(ctx),
            Line(
                "Simulation results are estimates based on available map data and simplified assumptions.",
            )
            .into_widget(ctx)
            .margin_below(15),
        ];
        for action in StarterAction::all() {
            rows.push(
                Widget::row(vec![
                    ctx.style()
                        .btn_outline
                        .text(action.label())
                        .build_def(ctx)
                        .centered_vert(),
                    Line(action.description()).into_widget(ctx).centered_vert(),
                ])
                .margin_below(10),
            );
        }
        rows.push(ctx.style().btn_plain.text("Start exploring").build_def(ctx));

        let panel = Panel::new_builder(Widget::col(rows)).build(ctx);
        <dyn SimpleState<_>>::new_state(panel, Box::new(StarterPanel { context }))
    }

    fn guidance(ctx: &mut EventCtx, action: StarterAction) -> Option<Transition<App>> {
        match action {
            StarterAction::ObserveTraffic => None,
            StarterAction::TestStreetChange => Some(Transition::Push(PopupMsg::new_state(
                ctx,
                "Test a street change",
                vec![
                    "Open Edit map to change lanes, speed limits, or intersection controls.",
                    "Save the change as a proposal, then run the simulation to inspect effects.",
                ],
            ))),
            StarterAction::CompareBeforeAfter => Some(Transition::Push(PopupMsg::new_state(
                ctx,
                "Compare before and after",
                vec![
                    "Save your street changes as a proposal before running the new scenario.",
                    "Use map layers and dashboards to compare travel times and traffic patterns.",
                ],
            ))),
        }
    }

    fn launch_sandbox(app: &mut App, action: Option<StarterAction>) -> Box<dyn State<App>> {
        let mode = default_gameplay_mode(app);
        SandboxMode::async_new(
            app,
            mode,
            Box::new(move |ctx, _| {
                action
                    .and_then(|action| StarterPanel::guidance(ctx, action))
                    .into_iter()
                    .collect()
            }),
        )
    }
}

impl SimpleState<App> for StarterPanel {
    fn on_click(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        x: &str,
        _: &mut Panel,
    ) -> Transition<App> {
        let action = StarterAction::all()
            .iter()
            .copied()
            .find(|action| action.label() == x);

        match self.context {
            StarterContext::BeforeSandbox => {
                Transition::Replace(StarterPanel::launch_sandbox(app, action))
            }
            StarterContext::InSandbox => {
                if let Some(action) = action {
                    if let Some(guidance) = StarterPanel::guidance(ctx, action) {
                        return Transition::Multi(vec![Transition::Pop, guidance]);
                    }
                }
                Transition::Pop
            }
        }
    }
}

pub(super) struct OpenProject;

impl OpenProject {
    pub(super) fn new_state() -> Box<dyn State<App>> {
        Box::new(OpenProject)
    }
}

impl State<App> for OpenProject {
    fn event(&mut self, _: &mut EventCtx, app: &mut App) -> Transition<App> {
        let mode = default_gameplay_mode(app);
        let load_mode = mode.clone();
        Transition::Replace(SandboxMode::async_new(
            app,
            mode,
            Box::new(move |ctx, app| {
                vec![Transition::Push(LoadEdits::new_state(ctx, app, load_mode))]
            }),
        ))
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, _: &mut GfxCtx, _: &App) {}
}

pub(super) fn default_gameplay_mode(app: &App) -> GameplayMode {
    GameplayMode::PlayScenario(
        app.primary.map.get_name().clone(),
        Scenario::default_scenario_for_map(app.primary.map.get_name()),
        Vec::new(),
    )
}
