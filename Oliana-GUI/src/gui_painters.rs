
use crate::*;


pub fn focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut inactive, mut border_color) in &mut text_input_query {
                if entity == interaction_entity {
                    inactive.0 = false;
                    *border_color = BORDER_COLOR_ACTIVE.into();
                } else {
                    inactive.0 = true;
                    *border_color = BORDER_COLOR_INACTIVE.into();
                }
            }
        }
    }
}


pub fn reset_scroll(
    q: Query<&Interaction, Changed<Interaction>>,
    mut scrolls_q: Query<&mut ScrollableContent>,
) {
    let Ok(mut scroll) = scrolls_q.get_single_mut() else {
        eprintln!("scrolls_q = returned None!");
        return;
    };
    for interaction in q.iter() {
        // eprintln!("interaction = {:?}", interaction);
        if interaction != &Interaction::Pressed {
            continue;
        }
        /*match action {
            ScrollButton::MoveToTop => scroll.scroll_to_top(),
            ScrollButton::MoveToBottom => scroll.scroll_to_bottom(),
        }*/
    }
}

