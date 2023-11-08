use bevy::prelude::*;
use corosensei::Coroutine;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, run_behavior)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let routine = Coroutine::new(|yielder, _input: ()| {
        let speed = 80.0;
        let displacements = [
            Vec2::new(200.0, 0.0),
            Vec2::new(0.0, 200.0),
            Vec2::new(-200.0, 0.0),
            Vec2::new(0.0, -200.0),
        ];
        for (i, d) in displacements.into_iter().enumerate() {
            println!("Starting walk {i}");
            yielder.suspend(YieldAction::Walk(WalkAction {
                speed,
                displacement: d,
                progress: 0.0,
            }));
        }
        println!("Finished walking");
    });
    commands
        .spawn(SpriteBundle {
            texture: asset_server.load("bevy_bird_dark.png"),
            ..default()
        })
        .insert(Behavior::new(routine));
}

#[derive(Component)]
struct Behavior {
    routine: Coroutine<(), YieldAction, ()>,
    current_action: Option<YieldAction>,
}
unsafe impl Send for Behavior {}

impl Behavior {
    fn new(routine: Coroutine<(), YieldAction, ()>) -> Self {
        Self {
            routine,
            current_action: None,
        }
    }

    fn take_action(&mut self) -> Option<YieldAction> {
        self.current_action.take().or_else(|| {
            // TODO (BUG report): without checking for done, we panic for trying
            // to resume finished routine, then get a segfault
            if self.routine.done() {
                None
            } else {
                self.routine.resume(()).as_yield()
            }
        })
    }

    fn continue_action(&mut self, action: YieldAction) {
        self.current_action = Some(action);
    }
}

#[derive(Clone, Debug)]
enum YieldAction {
    Walk(WalkAction),
}

#[derive(Clone, Debug)]
struct WalkAction {
    speed: f32,
    displacement: Vec2,
    progress: f32,
}

impl WalkAction {
    fn update(&mut self, dt: f32) -> Vec2 {
        let remaining_progress = 1.0 - self.progress;
        let max_move = remaining_progress * self.displacement.length();
        let delta_move = dt * self.speed;
        let move_length = if delta_move >= max_move {
            self.progress = 1.0;
            max_move
        } else {
            let progress = delta_move / self.displacement.length();
            self.progress += progress;
            delta_move
        };
        move_length * self.displacement.normalize()
    }
}

fn run_behavior(time: Res<Time>, mut behaviors: Query<(&mut Behavior, &mut Transform)>) {
    for (mut behavior, mut transform) in behaviors.iter_mut() {
        if let Some(mut action) = behavior.take_action() {
            // TODO: lock to exact displacement
            let action_complete = match &mut action {
                YieldAction::Walk(walk) => {
                    let translation = walk.update(time.delta_seconds());
                    transform.translation += translation.extend(0.0);
                    walk.progress >= 1.0
                }
            };
            if !action_complete {
                behavior.continue_action(action);
            } else {
                println!("Finished walk at {transform:?}");
            }
        }
    }
}
