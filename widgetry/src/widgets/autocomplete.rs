use abstutil::MultiMap;

use crate::{
    Choice, EventCtx, GfxCtx, Menu, Outcome, ScreenDims, ScreenPt, TextBox, Widget, WidgetImpl,
    WidgetOutput,
};

// TODO I don't even think we need to declare Clone...
// If multiple names map to the same data, all of the possible values will be returned
pub struct Autocomplete<T: Clone> {
    choices: Vec<(String, Vec<T>)>,
    num_search_results: usize,

    tb: TextBox,
    menu: Menu<()>,

    current_line: String,
    chosen_values: Option<Vec<T>>,
    show_results_when_empty: bool,
}

impl<T: 'static + Clone + Ord> Autocomplete<T> {
    pub fn new_widget(
        ctx: &mut EventCtx,
        raw_choices: Vec<(String, T)>,
        num_search_results: usize,
    ) -> Widget {
        Self::new_widget_with_options(ctx, raw_choices, num_search_results, 50, true)
    }

    pub fn new_compact_widget(
        ctx: &mut EventCtx,
        raw_choices: Vec<(String, T)>,
        num_search_results: usize,
        textbox_chars: usize,
    ) -> Widget {
        Self::new_widget_with_options(ctx, raw_choices, num_search_results, textbox_chars, false)
    }

    fn new_widget_with_options(
        ctx: &mut EventCtx,
        raw_choices: Vec<(String, T)>,
        num_search_results: usize,
        textbox_chars: usize,
        show_results_when_empty: bool,
    ) -> Widget {
        let mut grouped: MultiMap<String, T> = MultiMap::new();
        for (name, data) in raw_choices {
            grouped.insert(name, data);
        }
        let choices: Vec<(String, Vec<T>)> = grouped
            .consume()
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();

        let mut a = Autocomplete {
            choices,
            num_search_results,

            tb: TextBox::new(
                ctx,
                "autocomplete textbox".to_string(),
                textbox_chars,
                String::new(),
                true,
            ),
            menu: Menu::<()>::new(ctx, Vec::new()),

            current_line: String::new(),
            chosen_values: None,
            show_results_when_empty,
        };
        a.recalc_menu(ctx);
        Widget::new(Box::new(a))
    }
}

impl<T: 'static + Clone> Autocomplete<T> {
    pub fn take_final_value(&mut self) -> Option<Vec<T>> {
        self.chosen_values.take()
    }

    fn recalc_menu(&mut self, ctx: &mut EventCtx) {
        self.menu = Menu::new(
            ctx,
            matching_choice_labels(
                &self.choices,
                &self.current_line,
                self.num_search_results,
                self.show_results_when_empty,
            )
            .into_iter()
            .map(|label| Choice::new(label, ()))
            .collect(),
        );
    }
}

fn matching_choice_labels<T>(
    choices: &[(String, Vec<T>)],
    current_line: &str,
    num_search_results: usize,
    show_results_when_empty: bool,
) -> Vec<String> {
    if current_line.is_empty() && !show_results_when_empty {
        return Vec::new();
    }

    let mut labels = vec![format!("anything matching \"{}\"", current_line)];
    let query = current_line.to_ascii_lowercase();
    for (name, _) in choices {
        if name.to_ascii_lowercase().contains(&query) {
            labels.push(name.clone());
        }
        if labels.len() == num_search_results {
            break;
        }
    }
    // "anything matching" is silly if we've resolved to exactly one choice
    if labels.len() == 2 {
        labels.remove(0);
    }
    labels
}

impl<T: 'static + Clone> WidgetImpl for Autocomplete<T> {
    fn get_dims(&self) -> ScreenDims {
        let d1 = self.tb.get_dims();
        let d2 = self.menu.get_dims();
        ScreenDims::new(d1.width.max(d2.width), d1.height + d2.height)
    }

    fn set_pos(&mut self, top_left: ScreenPt) {
        self.tb.set_pos(top_left);
        self.menu.set_pos(ScreenPt::new(
            top_left.x,
            top_left.y + self.tb.get_dims().height,
        ));
    }

    fn event(&mut self, ctx: &mut EventCtx, output: &mut WidgetOutput) {
        self.tb.event(ctx, output);
        if self.tb.get_line() != self.current_line {
            // This will return Outcome::Changed to the caller with a dummy ID for the textbox
            self.current_line = self.tb.get_line();
            self.recalc_menu(ctx);
            output.redo_layout = true;
        } else {
            // Don't let the menu fill out the real outcome. Should we use Outcome::Changed to
            // indicate the autocomplete is finished, instead of the caller polling
            // autocomplete_done?
            let mut tmp_output = WidgetOutput::new();
            self.menu.event(ctx, &mut tmp_output);
            if let Outcome::Clicked(ref choice) = tmp_output.outcome {
                if choice.starts_with("anything matching") {
                    let query = self.current_line.to_ascii_lowercase();
                    let mut matches = Vec::new();
                    for (name, choices) in &self.choices {
                        if name.to_ascii_lowercase().contains(&query) {
                            matches.extend(choices.clone());
                        }
                    }
                    self.chosen_values = Some(matches);
                } else {
                    self.chosen_values = Some(
                        self.choices
                            .iter()
                            .find(|(name, _)| name == choice)
                            .unwrap()
                            .1
                            .clone(),
                    );
                }
            }
        }
    }

    fn draw(&self, g: &mut GfxCtx) {
        self.tb.draw(g);
        self.menu.draw(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_results_hide_empty_queries_and_respect_the_limit() {
        let choices = (0..8)
            .map(|n| (format!("city {}", n), vec![n]))
            .collect::<Vec<_>>();

        assert!(matching_choice_labels(&choices, "", 6, false).is_empty());

        let matches = matching_choice_labels(&choices, "city", 6, false);
        assert_eq!(matches.len(), 6);
        assert_eq!(matches[0], "anything matching \"city\"");
    }
}
