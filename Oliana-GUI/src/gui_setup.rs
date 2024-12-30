
use crate::*;

pub fn gui_setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    const STATUS_BAR_HEIGHT: f32 = 60.0;

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::End, // End here means "Bottom"
                    justify_content: JustifyContent::Start, // Start here means "Left"
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                ..default()
            },
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(80.0),
                        border: UiRect::all(Val::Px(2.0)),
                        padding: UiRect::all(Val::Px(2.0)),
                        margin: UiRect {
                            left: Val::Px(2.0),
                            top: Val::Px(2.0),
                            right: Val::Px(2.0),
                            bottom: Val::Px(STATUS_BAR_HEIGHT + 2.0),
                        },
                        ..default()
                    },
                    border_color: BORDER_COLOR_INACTIVE.into(),
                    background_color: BACKGROUND_COLOR.into(),
                    // Prevent clicks on the input from also bubbling down to the container
                    // behind it
                    focus_policy: bevy::ui::FocusPolicy::Block,
                    ..default()
                },
                TextInputBundle::default()
                    .with_text_style(TextStyle {
                        font_size: 32.0,
                        color: TEXT_COLOR,
                        ..default()
                    })
                    //.with_placeholder("Click to Type Text", None)
                    .with_inactive(true),
            ));

            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        right: Val::Px(4.0),
                        bottom: Val::Px(2.0),
                        height: Val::Px(STATUS_BAR_HEIGHT),
                        align_items: AlignItems::End, // Start here means "Top"
                        justify_content: JustifyContent::Start, // Start here means "Left"
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    // z_index: ZIndex::Global(1000), // This ensures the text floats _above_ the LLM response text.
                    ..default()
                },
            )).with_children(|node_bundle| {
                node_bundle.spawn((
                    TextBundle::from_section(
                        // Accepts a `String` or any type that converts into a `String`, such as `&str`
                        "127.0.0.1:9050",
                        TextStyle {
                            // This font is loaded and will be used instead of the default font.
                            // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 14.0,
                            color: TEXT_COLOR,
                            ..default()
                        },
                    ) // Set the justification of the Text
                    .with_text_justify(JustifyText::Left)
                    // Set the style of the TextBundle itself.
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        right: Val::Px(1.0),
                        bottom: Val::Px(1.0),
                        //width: Val::Px(400.0),
                        margin: UiRect::all(Val::Px(1.0)),
                        //border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }),
                    Server_URL,
                ));
            });

        });

    commands.spawn(( // TODO move me upstairs?
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(STATUS_BAR_HEIGHT + 56.0),
                align_items: AlignItems::Start, // Start here means "Top"
                justify_content: JustifyContent::Start, // Start here means "Left"
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            border_color: BORDER_COLOR_INACTIVE.into(),
            background_color: LLM_OUTPUT_BACKGROUND_COLOR.into(),
            ..default()
        },
        ScrollView::default(),
    ))
    .with_children(|scroll_area| {
        scroll_area.spawn((
            TextBundle::from_section(
                // Accepts a `String` or any type that converts into a `String`, such as `&str`
                "Hello\nOliana!",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    ..default()
                },
            ) // Set the justification of the Text
            .with_text_justify(JustifyText::Left)
            // Set the style of the TextBundle itself.
            .with_style(Style {
                align_self: AlignSelf::Stretch,
                /*position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(56.0),*/
                // min_height: Val::Px(900.0),
                margin: UiRect::all(Val::Px(4.0)),
                //border: UiRect::all(Val::Px(5.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            }),
            LLM_ReplyText,
            ScrollableContent::default(),
        ));
    });


}
