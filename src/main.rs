#![feature(plugin)]
#![plugin(embed)]
#[macro_use]
extern crate glium;
extern crate gif;
// noinspection SpellCheckingInspection
const LIFE_PALETTE: [[u8; 3]; 4] =
[[0x23, 0x07, 0x03], [0x6d, 0x57, 0x1e], [0x9a, 0xc1, 0x6e], [0xd7, 0xf4, 0xd9]];
const DEATH_PALETTE: [[u8; 3]; 4] =
[[0x03, 0x1b, 0x1e], [0x1f, 0x2a, 0x54], [0x90, 0x70, 0xa3], [0xea, 0xd7, 0xe4]];
fn main() {
    use glium::{DisplayBuild, Surface};
    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(160, 144)
        .with_title(format!("gbjam5"))
        .build_glium()
        .unwrap();

    let program = program!(&display,
    150 => {
        vertex: include_str!("sprite.vert"),
        fragment: include_str!("sprite.frag"),
        geometry: include_str!("sprite.geom"),
    })
        .unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::Points);
    let vertex_buffer = glium::VertexBuffer::new(&display, &vec![Vertex { position: [0.0, 0.0] }])
        .unwrap();
    let expanded = [expand_palette(&LIFE_PALETTE), expand_palette(&DEATH_PALETTE)];
    let mut bg = [expanded[0].1, expanded[1].1];
    let mut palettes =
        [glium::texture::SrgbTexture1d::new(&display, expanded[1].0.clone()).unwrap(),
         glium::texture::SrgbTexture1d::new(&display, expanded[1].0.clone()).unwrap()];
    let params =
        glium::DrawParameters { blend: glium::Blend::alpha_blending(), ..Default::default() };
    let step_time = Duration::from_millis(20);
    let mut game = Game::load(&display);
    let mut input = Default::default();
    loop {
        let instant = Instant::now();
        let mut target = display.draw();
        let sprites = game.step(&input);
        let palette = game.palette_id;
        if game.palette_changed {
            let expanded = [expand_palette(&game.palettes[0]), expand_palette(&game.palettes[1])];
            bg = [expanded[0].1, expanded[1].1];
            //println!("{:?}", expanded);
            palettes =
                [glium::texture::SrgbTexture1d::new(&display, expanded[0].0.clone()).unwrap(),
                 glium::texture::SrgbTexture1d::new(&display, expanded[1].0.clone()).unwrap()];
        }
        target.clear_color_srgb(bg[palette].0, bg[palette].1, bg[palette].2, 0.0);
        for texture in sprites {
            let uniforms = uniform! {
            tex: texture.0.sampled(),
            palette: &palettes[palette],
            offset: [texture.1[0], texture.1[1]],
            flip: texture.2
        };
            target.draw(&vertex_buffer, &indices, &program, &uniforms, &params)
                .unwrap();
        }
        target.finish().unwrap();
        input.start = false;
        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                glium::glutin::Event::KeyboardInput(state, key, code) => {
                    //println!("{:?}, {:?}, {:?}", state, key, code);
                    let state = state == glium::glutin::ElementState::Pressed;
                    use glium::glutin::VirtualKeyCode::*;
                    match code {
                        Some(Z) => input.b = state,
                        Some(X) => input.a = state,
                        Some(Left) => input.left = state,
                        Some(Right) => input.right = state,
                        Some(Up) => input.up = state,
                        Some(Down) => input.down = state,
                        Some(Return) => input.start = state,
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        let elapsed = instant.elapsed();
        if step_time > elapsed {
            std::thread::sleep(step_time - elapsed);
        }
    }
}

fn expand_palette(palette: &[[u8; 3]; 4]) -> (Vec<(u8, u8, u8, u8)>, (f32, f32, f32)) {
    let mut vec = vec![];
    for i in 0..4 {
        vec.push((palette[i][0], palette[i][1], palette[i][2], 0xFF));
    }
    vec.push((0, 0, 0, 0));
    (vec,
     (palette[3][0] as f32 / 256.0, palette[3][1] as f32 / 256.0, palette[3][2] as f32 / 256.0))
}

#[derive(Default)]
struct Input {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    b: bool,
    a: bool,
    start: bool,
}

use std::collections::HashMap;

struct Game {
    textures: HashMap<String, Texture>,
    state: GameState,
    levels: HashMap<String, String>,
    palette_id: usize,
    palettes: [[[u8; 3]; 4]; 2],
    palette_changed: bool,
}

enum GameState {
    MENU,
    LEVEL(Level),
}

type TileMap = Vec<Vec<(Rc<Tile>, u8)>>;
#[derive(Default)]
struct Level {
    tile_map: [TileMap; 2],
    tile_sprites: Vec<Sprite>,
    entities: Vec<Entity>,
    wraparound: bool,
    width: usize,
    height: usize,
    version: usize,
}

#[derive(Debug)]
struct Tile {
    sprite_id: usize,
    tile_type: TileType,
}

impl Tile {
    fn is_solid(&self) -> bool {
        match self.tile_type {
            TileType::Background => false,
            TileType::Checkpoint => false,
            TileType::Door(_) => false,
            TileType::Switch => false,
            TileType::Solid => true,
            TileType::SwitchBlock => true,
        }
    }
}

#[derive(Debug)]
enum TileType {
    Background,
    Solid,
    Door(String),
    Checkpoint,
    Switch,
    SwitchBlock,
}

impl Default for TileType {
    fn default() -> Self {
        TileType::Background
    }
}

struct Entity {
    x: usize,
    y: usize,
    facing: bool,
    entity_type: EntityType,
}

enum EntityType {
    Player(Player),
}

struct Player {
    state: PlayerState,
    sprites: [PlayerSprites; 2],
}

struct PlayerSprites {
    walking: Sprite,
    standing: Sprite,
    falling: Sprite,
    jumping: Sprite,
}

#[derive(Eq, PartialEq)]
enum PlayerState {
    Walking,
    Standing,
    Falling,
    Jumping(u8),
}

impl Game {
    pub fn load<F>(facade: &F) -> Game
        where F: glium::backend::Facade
    {
        let mut textures = HashMap::new();
        let mut levels = HashMap::new();
        for (name, content) in embed!("assets") {
            let name = String::from_utf8(name).unwrap().replace(r"\", "/");
            //println!("{}", name);
            if name.ends_with(".gif") {
                textures.insert(name[..name.len() - 4].to_string(),
                                Sprite::load(facade, &content));
            } else if name.starts_with("levels/") {
                levels.insert(name[7..name.len() - 4].to_string(),
                              String::from_utf8(content).unwrap());
            }
        }
        Game {
            palette_changed: true,
            textures: textures,
            levels: levels,
            state: GameState::MENU,
            palette_id: 0,
            palettes: [LIFE_PALETTE, DEATH_PALETTE],
        }
    }

    fn load_level(&self, string: &str) -> Level {
/*        println!("{}", string);
        for key in self.textures.keys() {
            println!("{:?}", key)
        }*/
        let mut lines = string.lines();
        let mut tile_mappings = HashMap::new();
        let mut sprite_map = HashMap::new();
        let mut tile_sprites = vec![];
        sprite_map.insert("tiles/blank", 0);
        tile_sprites.push(Sprite::new(self.textures.get("tiles/blank").unwrap()));
        tile_mappings.insert(' ',
                             Rc::new(Tile {
                                 sprite_id: 0,
                                 tile_type: TileType::Background,
                             }));
        while let Some(line) = lines.next() {
            if line == "ENTITY" {
                break;
            }
            let mut values = line.split(',');
            let character = values.next().unwrap().chars().next().unwrap();
            let filename = values.next().unwrap();
            let tile = Tile {
                sprite_id: {
                    *sprite_map.entry(filename).or_insert({
                        if let Some(texture) = self.textures.get(filename) {
                            tile_sprites.push(Sprite::new(texture));
                            tile_sprites.len() - 1
                        } else {
                            0
                        }
                    })
                },
                tile_type: match values.next().unwrap().to_lowercase().as_ref() {
                    "background" => TileType::Background,
                    "solid" => TileType::Solid,
                    "door" => TileType::Door(values.next().unwrap().to_string()),
                    "checkpoint" => TileType::Checkpoint,
                    "switch" => TileType::Switch,
                    "switchblock" => TileType::SwitchBlock,
                    _ => TileType::Background,
                },
            };
            tile_mappings.insert(character, Rc::new(tile));
        }
        while let Some(line) = lines.next() {
            if line == "LEVEL" {
                break;
            }
        }
        let mut values = lines.next().unwrap().split(',');
        let width = values.next().unwrap().parse().unwrap();
        let height = values.next().unwrap().parse().unwrap();
        let mut wraparound = false;
        while let Some(value) = values.next() {
            match value.to_lowercase().as_ref() {
                "wraparound" => {
                    wraparound = true;
                }
                _ => (),
            }
        }
        let mut tile_maps = vec![];
        for _ in 0..2 {
            let mut tile_map = vec![];
            for _ in 0..height {
                let mut chars = lines.next().unwrap_or("").chars();
                let mut row = vec![];
                for _ in 0..width {
                    row.push((tile_mappings.get(&chars.next().unwrap_or(' '))
                        .unwrap_or(tile_mappings.get(&' ').unwrap())
                        .clone(),
                              0));
                }
                tile_map.push(row);
            }
            tile_map.reverse();
            //println!("{:?}", tile_map);
            tile_maps.push(tile_map);
        }
        let mut entities = Vec::new();
        entities.push(Entity {
                x: 0,
                y: 5 * 16,
                facing: true,
                entity_type: EntityType::Player(Player {
                    state: PlayerState::Standing,
                    sprites: [PlayerSprites {
                        walking: Sprite::new(&self.textures["player/MonsterWalk"]),
                        standing: Sprite::new(&self.textures["player/MonsterStand"]),
                        falling: Sprite::new(&self.textures["player/MonsterFalling"]),
                        jumping: Sprite::new(&self.textures["player/MonsterJump"]),
                    },
                        PlayerSprites {
                            walking: Sprite::new(&self.textures["player/HumanWalk"]),
                            standing: Sprite::new(&self.textures["player/HumanStand"]),
                            falling: Sprite::new(&self.textures["player/HumanFalling"]),
                            jumping: Sprite::new(&self.textures["player/HumanJump"]),
                        }],
                }),
            });
        Level {
            tile_map: [tile_maps.remove(0), tile_maps.remove(0)],
            tile_sprites: tile_sprites,
            width: width,
            height: height,
            wraparound: wraparound,
            entities: entities,
            version: 0
        }
    }

    pub fn step(&mut self, input: &Input) -> Vec<(Rc<Texture2d>, [f32; 2], bool)> {
        let mut sprites = vec![];
        match self.state {
            GameState::MENU => {
                self.state =
                    GameState::LEVEL(self.load_level(self.levels.get("Death_Jumping_Level").unwrap()));
            }
            GameState::LEVEL(ref mut level) => {
                if input.start {
                    level.version = level.version ^ 1;
                }
                self.palette_changed = false;
                let textures: Vec<Rc<Texture2d>> =
                    level.tile_sprites.iter_mut().map(|sprite| sprite.texture()).collect();
                for y in 0..level.height {
                    for x in 0..level.width {
                        if level.tile_map[level.version][y][x].0.sprite_id != 0 {
                            sprites.push((textures[level.tile_map[level.version][y][x].0.sprite_id].clone(),
                                          [(x * 16) as f32, (y * 16) as f32], false))
                        }
                    }
                }
                for entity in level.entities.iter_mut() {
                    if let Some(sprite) = match entity.entity_type {
                        EntityType::Player(ref mut player) => {
                            if !(level.tile_map[level.version][(entity.y - 1) / 16][entity.x / 16].0.is_solid() || level.tile_map[level.version][(entity.y - 1) / 16][(entity.x + 15) / 16].0.is_solid()) {
                                entity.y -= 2;
                                player.state = PlayerState::Falling
                            } else {
                                player.state = PlayerState::Standing
                            }
                            Option::Some(match player.state {
                                PlayerState::Falling => player.sprites[level.version].falling.texture(),
                                PlayerState::Standing => player.sprites[level.version].standing.texture(),
                                PlayerState::Jumping(_) => player.sprites[level.version].jumping.texture(),
                                PlayerState::Walking => player.sprites[level.version].walking.texture(),
                            })
                        }
                    } {
                        sprites.push((sprite, [entity.x as f32, entity.y as f32], entity.facing))
                    }
                }
                self.palette_id = level.version;
            }
        }
        sprites
    }
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

use std::time::Duration;
use glium::texture::Texture2d;

use std::time::Instant;
use std::rc::Rc;
type Texture = Rc<Vec<(Rc<Texture2d>, Duration)>>;

struct Animator {
    index: usize,
    instant: Instant,
}
struct Sprite {
    texture: Texture,
    animator: Option<Animator>,
}

impl Sprite {
    pub fn load<F>(facade: &F, file: &[u8]) -> Texture
        where F: glium::backend::Facade
    {
        use gif::SetParameter;
        let mut decoder = gif::Decoder::new(file);
        decoder.set(gif::ColorOutput::Indexed);
        let mut decoder = decoder.read_info().unwrap();

        let mut texture: Vec<(Rc<Texture2d>, Duration)> = vec![];

        while let Some(frame) = decoder.read_next_frame().unwrap() {
            texture.push((Rc::new(Texture2d::new(facade,
                                                          glium::texture::RawImage2d {
                                                              data: frame.buffer.clone(),
                                                              width: frame.width as u32,
                                                              height: frame.height as u32,
                                                              format:
                                                                  glium::texture::ClientFormat::U8,
                                                          }).unwrap()),
                          Duration::from_millis(10 * frame.delay as u64)));
            //println!("{:?}", frame);
        }
        Rc::new(texture)
    }

    pub fn new(texture: &Texture) -> Sprite {
        Sprite {
            texture: texture.clone(),
            animator: if texture.len() == 1 {
                None
            } else {
                Some(Animator {
                    index: 0,
                    instant: Instant::now(),
                })
            },
        }
    }

    pub fn reset(&mut self) {
        if let Some(ref mut animator) = self.animator {
            animator.index = 0;
            animator.instant = Instant::now();
        }
    }

    pub fn texture(&mut self) -> Rc<Texture2d> {
        match self.animator {
            Some(ref mut animator) => {
                if animator.instant.elapsed() > self.texture[animator.index].1 {
                    animator.instant = Instant::now();
                    animator.index = (animator.index + 1) % self.texture.len();
                }
                self.texture[animator.index].0.clone()
            }
            None => self.texture[0].0.clone(),
        }
    }
}
