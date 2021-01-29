use arche_tape::{EcsId, EcsIds, World};
use graphics::{Camera, DrawParams, Mesh, Rectangle, ShapeStyle};
use input::get_mouse_position;
use rand::random;
use tetra::{
    graphics::{self, Color},
    input,
    math::Vec2,
};
use tetra::{Context, ContextBuilder, State};

fn main() -> tetra::Result {
    ContextBuilder::new("Rectangel", 1280, 720)
        .build()?
        .run(GameState::new)
}

const NOM_GROWTH: u32 = 2;

const ENEMY_INIT_SIZE: u32 = 2;
const ENEMY_MAX_SIZE: u32 = 25;
const ENEMY_MAX_SPEED: f32 = 10.;
const ENEMY_SPEED: f32 = 1.5;

const PLAYER_MIN_SIZE: u32 = 8;
const PLAYER_INIT_SIZE: u32 = 14;
const PLAYER_MAX_SIZE: u32 = 20;
const PLAYER_I_FRAMES: u32 = 30;

const RESTART_TIME: u32 = 120;

const POWERUP_TIMER: u32 = 10 * 60;
const POWERUP_DURATION: u32 = 4 * 60;
const POWERUP_SIZE: u32 = 20;

const ENEMY_SPAWN_TIMER_RANGE: (u32, u32) = (4 * 60, 7 * 60);
const ENEMY_SPAWN_AMOUNT_RANGE: (u32, u32) = (6, 14);

const ENEMY_RUN_FROM_DISTANCE: f32 = 300.;

struct Powerup;
struct Player;
struct Enemy;

struct Velocity {
    vel: Vec2<f32>,
}

struct Nommer {
    size: u32,
    i_frames: u32,

    powerup_remaining: u32,
}

struct Pos(Vec2<f32>);

impl Pos {
    fn clamp_pos(&mut self) {
        self.0.x = self.0.x.min(640.0);
        self.0.x = self.0.x.max(-640.0);
        self.0.y = self.0.y.min(360.0);
        self.0.y = self.0.y.max(-360.0);
    }
}

#[derive(Clone)]
struct Renderable {
    color: Color,
    mesh: Mesh,
}

struct GameState {
    tick: u128,
    world: World,
    camera: Camera,
    player: EcsId,
    nommer_mesh: Mesh,

    game_over: bool,
    restart_timer: u32,

    powerup_timer: u32,

    enemy_spawn_timer: u32,
}

impl GameState {
    fn new(ctx: &mut Context) -> tetra::Result<Self> {
        let nommer_mesh = Mesh::rectangle(ctx, ShapeStyle::Fill, Rectangle::new(-1., -1., 2., 2.))?;

        let mut world = World::new();

        let player = world
            .spawn()
            .with(Player)
            .with(Pos(get_mouse_position(ctx)))
            .with(Nommer {
                size: PLAYER_INIT_SIZE,
                i_frames: 0,
                powerup_remaining: 0,
            })
            .with(Renderable {
                color: Color::WHITE,
                mesh: nommer_mesh.clone(),
            })
            .build();

        Ok(Self {
            tick: 0,
            world,
            camera: Camera::with_window_size(ctx),
            player,
            nommer_mesh,

            game_over: false,
            restart_timer: 0,

            powerup_timer: POWERUP_TIMER,
            enemy_spawn_timer: 0,
        })
    }
}

impl State for GameState {
    fn update(&mut self, ctx: &mut Context) -> tetra::Result {
        self.tick += 1;

        if self.game_over {
            let mut to_despawn = Vec::new();
            for (e, _) in self
                .world
                .query::<(EcsIds, &Enemy)>()
                .iter()
                .filter(|&(e, _)| e != self.player)
            {
                to_despawn.push(e);
            }
            for e in to_despawn {
                self.world.despawn(e);
            }

            if self.restart_timer == 0 {
                *self = GameState::new(ctx)?;
                return Ok(());
            }
            self.restart_timer -= 1;
        }

        let entities = self
            .world
            .query::<(EcsIds, &Nommer)>()
            .iter()
            .filter(|&(id, _)| !self.world.has_component::<Renderable>(id))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        for entity in entities {
            let renderable = Renderable {
                color: Color::BLUE,
                mesh: self.nommer_mesh.clone(),
            };
            self.world.add_component(entity, renderable);
        }

        let player_pos = self.world.get_component_mut::<Pos>(self.player).unwrap();
        player_pos.0 = get_mouse_position(ctx) - Vec2::new(640., 360.);
        player_pos.clamp_pos();
        let player_pos = player_pos.0.clone();
        let player_nommer = self.world.get_component_mut::<Nommer>(self.player).unwrap();
        let player_size = player_nommer.size;
        let player_powerup = player_nommer.powerup_remaining;

        if self.tick % 15 == 0 {
            for (_, nommer) in self.world.query::<(&Enemy, &mut Nommer)>().iter() {
                nommer.size += 1;
                nommer.size = nommer.size.min(ENEMY_MAX_SIZE);
            }
        }

        for (nommer, pos, velocity, _) in self
            .world
            .query::<(&Nommer, &mut Pos, &mut Velocity, &Enemy)>()
            .iter()
        {
            // NaN shenanigans make this necessary
            if player_pos == pos.0 {
                continue;
            }

            if player_size < nommer.size && player_powerup == 0 {
                velocity.vel += (player_pos - pos.0).normalized() * ENEMY_SPEED;
            } else if (player_pos - pos.0).magnitude() < ENEMY_RUN_FROM_DISTANCE {
                velocity.vel -= (player_pos - pos.0).normalized() * ENEMY_SPEED;
            } else {
                velocity.vel +=
                    Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5).normalized();
            }

            if velocity.vel.magnitude() > ENEMY_MAX_SPEED {
                velocity.vel = velocity.vel.normalized() * ENEMY_MAX_SPEED;
            }

            pos.0 += velocity.vel;
            pos.clamp_pos();
        }

        let mut resistances = Vec::new();
        for (e1, pos_1, nommer_1, _, _) in self
            .world
            .query::<(EcsIds, &Pos, &Nommer, &Enemy, &Velocity)>()
            .iter()
            .filter(|(_, _, nommer, _, _)| nommer.size > player_size)
        {
            for (_, pos_2, nommer_2, _) in self
                .world
                .query::<(EcsIds, &Pos, &Nommer, &Enemy)>()
                .iter()
                .filter(|&(e2, _, _, _)| e2 != e1)
            {
                let pos_diff = pos_1.0 - pos_2.0;
                let pos_diff = Vec2::new(pos_diff.x.abs(), pos_diff.y.abs());
                let pos_diff = Vec2::new(
                    pos_diff.x - (nommer_1.size + 10) as f32 - (nommer_2.size + 10) as f32,
                    pos_diff.y - (nommer_1.size + 10) as f32 - (nommer_2.size + 10) as f32,
                );

                if pos_diff.x < 0. && pos_diff.y < 0. {
                    let vel = if pos_1.0 == pos_2.0 {
                        let mut x = (random::<f32>() * 2.0) - 1.0;
                        let mut y = (random::<f32>() * 2.0) - 1.0;
                        if x == 0. {
                            if random() {
                                x = -0.1;
                            } else {
                                x = 0.1;
                            }
                        }
                        if y == 0. {
                            if random() {
                                y = -0.1;
                            } else {
                                y = 0.1;
                            }
                        }

                        Vec2::new(x, y).normalized()
                    } else {
                        (pos_1.0 - pos_2.0).normalized()
                    };
                    let vel = vel * pos_diff.magnitude() * 0.01;
                    resistances.push((e1, vel));
                }
            }
        }
        for (entity, velocity) in resistances {
            self.world
                .get_component_mut::<Velocity>(entity)
                .unwrap()
                .vel += velocity;
        }

        for (nommer,) in self.world.query::<(&mut Nommer,)>().iter() {
            if nommer.i_frames > 0 {
                nommer.i_frames -= 1;
            }
        }

        let mut nommed = Vec::new();
        if let Some((p_pos, p_nommer, _)) =
            self.world.query::<(&Pos, &Nommer, &Player)>().iter().next()
        {
            for (e, pos, nommer, _) in self.world.query::<(EcsIds, &Pos, &Nommer, &Enemy)>().iter()
            {
                let diff_pos = p_pos.0 - pos.0;
                let diff_pos = f32::max(diff_pos.x.abs(), diff_pos.y.abs());

                if (p_nommer.size > nommer.size || p_nommer.powerup_remaining > 0)
                    && nommer.i_frames == 0
                {
                    if diff_pos < (p_nommer.size + nommer.size) as f32 + 2. {
                        nommed.push(e);
                    }
                }

                if nommer.size > p_nommer.size && p_nommer.i_frames == 0 {
                    if diff_pos < (p_nommer.size + nommer.size) as f32 + 2. {
                        nommed.push(self.player);
                    }
                }
            }
        }
        for entity in nommed {
            if entity == self.player {
                let nommer = self.world.get_component_mut::<Nommer>(self.player).unwrap();
                if nommer.i_frames == 0 && nommer.powerup_remaining == 0 {
                    nommer.size -= NOM_GROWTH;
                    nommer.i_frames = PLAYER_I_FRAMES;
                    if nommer.size < PLAYER_MIN_SIZE {
                        self.game_over = true;
                        self.restart_timer = RESTART_TIME;
                    }
                }
            } else {
                let nommer = self.world.get_component_mut::<Nommer>(self.player).unwrap();
                if nommer.powerup_remaining == 0 {
                    nommer.size += NOM_GROWTH;
                    nommer.size = u32::min(nommer.size, PLAYER_MAX_SIZE);
                }
                self.world.despawn(entity);
            }
        }

        if self.powerup_timer == 0 {
            let x = rand::random::<f32>() * 1280.;
            let y = rand::random::<f32>() * 720.;
            let pos = Vec2::new(x, y) - Vec2::new(640., 360.);

            self.world
                .spawn()
                .with(Powerup)
                .with(Pos(pos))
                .with(Renderable {
                    mesh: self.nommer_mesh.clone(),
                    color: Color::GREEN,
                })
                .build();

            self.powerup_timer = POWERUP_TIMER;
        }
        if !self.game_over {
            self.powerup_timer -= 1;
        }
        let mut powerups_collected = Vec::new();
        if let Some((_, player_pos, player_nommer)) =
            self.world.query::<(&Player, &Pos, &Nommer)>().iter().next()
        {
            for (powerup_entity, _, powerup_pos) in
                self.world.query::<(EcsIds, &Powerup, &Pos)>().iter()
            {
                let pos_diff = player_pos.0 - powerup_pos.0;
                let pos_diff = Vec2::new(pos_diff.x.abs(), pos_diff.y.abs());
                let pos_diff = f32::max(pos_diff.x, pos_diff.y);

                if (pos_diff as u32) < player_nommer.size + 5 + POWERUP_SIZE {
                    powerups_collected.push(powerup_entity);
                }
            }
        }
        for id in powerups_collected {
            self.world.despawn(id);
            let nommer = self.world.get_component_mut::<Nommer>(self.player).unwrap();
            nommer.powerup_remaining = POWERUP_DURATION;
        }
        for (nommer,) in self.world.query::<(&mut Nommer,)>().iter() {
            if nommer.powerup_remaining > 0 {
                nommer.powerup_remaining -= 1;
            }
        }

        if self.enemy_spawn_timer == 0 {
            let percent = random::<f32>();
            let amount = percent * (ENEMY_SPAWN_AMOUNT_RANGE.1 - ENEMY_SPAWN_AMOUNT_RANGE.0) as f32;
            let amount = amount + ENEMY_SPAWN_AMOUNT_RANGE.0 as f32;
            for _ in 0..(amount as u32) {
                let x = rand::random::<f64>() * 1280.;
                let y = rand::random::<f64>() * 720.;
                self.world
                    .spawn()
                    .with(Enemy)
                    .with(Pos(Vec2::new(x as _, y as _) - Vec2::new(640., 360.)))
                    .with(Nommer {
                        size: ENEMY_INIT_SIZE,
                        i_frames: 0,
                        powerup_remaining: 0,
                    })
                    .with(Velocity { vel: Vec2::zero() })
                    .build();
            }

            let percent = random::<f32>();
            let spawn_timer =
                percent * (ENEMY_SPAWN_TIMER_RANGE.1 - ENEMY_SPAWN_TIMER_RANGE.0) as f32;
            let spawn_timer = spawn_timer + ENEMY_SPAWN_TIMER_RANGE.0 as f32;
            self.enemy_spawn_timer = spawn_timer as _;
        }
        if !self.game_over {
            self.enemy_spawn_timer -= 1;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        // Cornflower blue, as is tradition
        graphics::clear(ctx, Color::rgb(0.392, 0.584, 0.929));

        self.camera.update();
        graphics::set_transform_matrix(ctx, self.camera.as_matrix());

        for (pos, renderable, _) in self.world.query::<(&Pos, &Renderable, &Powerup)>().iter() {
            graphics::draw(
                ctx,
                &renderable.mesh,
                DrawParams::new()
                    .position(pos.0)
                    .scale(Vec2::new(POWERUP_SIZE as _, POWERUP_SIZE as _))
                    .color(renderable.color),
            )
        }

        let player_size = self
            .world
            .get_component_mut::<Nommer>(self.player)
            .unwrap()
            .size;

        for (_, pos, nommer, renderable) in self
            .world
            .query::<(&Enemy, &Pos, &Nommer, &Renderable)>()
            .iter()
        {
            draw_nommer(ctx, pos, nommer, renderable, player_size);
        }

        if let Some((_, pos, nommer, renderable)) = self
            .world
            .query::<(&Player, &Pos, &Nommer, &Renderable)>()
            .iter()
            .next()
        {
            let mut color = renderable.color;
            if nommer.i_frames > 5 {
                color = Color::RED;
            }

            if nommer.powerup_remaining > 20 {
                color = Color::GREEN;
            }

            graphics::draw(
                ctx,
                &renderable.mesh,
                DrawParams::new()
                    .position(pos.0)
                    .scale(Vec2::new(nommer.size as _, nommer.size as _))
                    .color(color),
            );
        }

        Ok(())
    }
}

fn draw_nommer(
    ctx: &mut Context,
    pos: &Pos,
    nommer: &Nommer,
    renderable: &Renderable,
    player_size: u32,
) {
    let mut color = renderable.color;
    if nommer.size + NOM_GROWTH <= player_size {
        color = Color::rgb(0.2, 0.2, 0.6);
    }

    graphics::draw(
        ctx,
        &renderable.mesh,
        DrawParams::new()
            .position(pos.0)
            .scale(Vec2::new(nommer.size as _, nommer.size as _))
            .color(color),
    );
}
