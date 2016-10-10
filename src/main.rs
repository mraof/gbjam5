#![feature(plugin)]
#![plugin(embed)]
#[macro_use]
extern crate glium;
extern crate gif;
extern crate rodio;
const LIFE_PALETTE: [[u8; 3]; 4] = [[0x23, 0x07, 0x03], [0x6d, 0x57, 0x1e], [0x9a, 0xc1, 0x6e], [0xd7, 0xf4, 0xd9]];
const DEATH_PALETTE: [[u8; 3]; 4] = [[0x03, 0x1b, 0x1e], [0x1f, 0x2a, 0x54], [0x90, 0x70, 0xa3], [0xea, 0xd7, 0xe4]];
fn main() {
    use glium::{DisplayBuild, Surface};
    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(160, 144)
        .with_title(format!("gbjam5"))
        .build_glium()
        .unwrap();

    let program = program!(&display,
    140 => {
        vertex: include_str!("sprite.vert"),
        fragment: include_str!("sprite.frag"),
    })
        .unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);
    let vertex_buffer = glium::VertexBuffer::immutable(&display,
                                                       &vec![Vertex { position: [0.0, 0.0] }, Vertex { position: [1.0, 0.0] }, Vertex { position: [0.0, 1.0] }, Vertex { position: [1.0, 1.0] }])
        .unwrap();
    let (expanded, mut bg) = expand_palettes([&LIFE_PALETTE, &DEATH_PALETTE]);
    let mut palettes = Vec::new();
    for palette in &expanded {
        palettes.push(glium::texture::SrgbTexture1d::new(&display, palette.clone()).unwrap());
    }
    let params = glium::DrawParameters { blend: glium::Blend::alpha_blending(), ..Default::default() };
    let step_time = Duration::from_millis(20);
    let mut game = Game::load(&display);
    let mut input = Default::default();
    loop {
        let instant = Instant::now();
        let mut target = display.draw();
        let sprites = game.step(&input);
        let palette = game.palette_id;
        if game.palette_changed {
            let (expanded, new_bg) = expand_palettes([&game.palettes[0], &game.palettes[1]]);
            bg = new_bg;
            palettes.clear();
            for palette in &expanded {
                palettes.push(glium::texture::SrgbTexture1d::new(&display, palette.clone()).unwrap());
            }
        }
        target.clear_color_srgb(bg[palette].0, bg[palette].1, bg[palette].2, 0.0);
        for texture in sprites {
            let uniforms = uniform! {
                tex: texture.0.sampled(),
                palette: &palettes[palette],
                offset: [texture.1[0] as f32, texture.1[1] as f32],
                flip: texture.2
            };
            target.draw(&vertex_buffer, &indices, &program, &uniforms, &params)
                .unwrap();
        }
        target.finish().unwrap();
        input.start = false;
        input.a = false;
        input.b = false;
        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                glium::glutin::Event::KeyboardInput(state, _, code) => {
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

/// Expands the palettes and creates the transition palettes
/// L0 L1 L2 L3 T
/// D0 D1 D2 D3 T
/// L0 L1 L2 L2 T
/// L0 L1 L1 L1 T
/// D0 L0 T  T  T
/// D0 D1 D1 D1 T
/// D0 D1 D2 D2 T
fn expand_palettes(palettes: [&[[u8; 3]; 4]; 2]) -> ([(Vec<(u8, u8, u8, u8)>); 7], [(f32, f32, f32); 7]) {
    let mut expanded = Vec::new();
    let mut clear_colors = Vec::new();
    for p in 0..2 {
        let palette = palettes[p];
        let mut vec = Vec::new();
        for i in 0..4 {
            vec.push((palette[i][0], palette[i][1], palette[i][2], 0xFF));
        }
        vec.push((0, 0, 0, 0));
        expanded.push(vec);
        clear_colors.push((palette[3][0] as f32 / 256.0, palette[3][1] as f32 / 256.0, palette[3][2] as f32 / 256.0));
    }
    ([expanded[0].clone(),
      expanded[1].clone(),
      vec![expanded[0][0], expanded[0][1], expanded[0][2], expanded[0][2], (0, 0, 0, 0)],
      vec![expanded[0][0], expanded[0][1], expanded[0][1], expanded[0][1], (0, 0, 0, 0)],
      vec![expanded[1][0], expanded[0][0], (0, 0, 0, 0), (0, 0, 0, 0), (0, 0, 0, 0)],
      vec![expanded[1][0], expanded[1][1], expanded[1][1], expanded[1][1], (0, 0, 0, 0)],
      vec![expanded[1][0], expanded[1][1], expanded[1][2], expanded[1][2], (0, 0, 0, 0)]],
     [clear_colors[0], clear_colors[1], clear_colors[0], clear_colors[0], clear_colors[1], clear_colors[1], clear_colors[1]])
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
use std::sync::mpsc::{Sender,channel};

struct Game {
    textures: HashMap<String, Texture>,
    font: HashMap<char, Sprite>,
    state: GameState,
    levels: HashMap<String, String>,
    palette_id: usize,
    palettes: [[[u8; 3]; 4]; 2],
    palette_changed: bool,
    music: Sender<f32>,
}

enum GameState {
    Menu(Menu),
    Level(Level),
}

struct Menu {
    selection: u8,
}

type TileMap = Vec<Vec<Rc<Tile>>>;

struct Level {
    tile_map: [TileMap; 2],
    tile_sprites: Vec<Sprite>,
    entities: Vec<Entity>,
    wraparound: bool,
    width: i32,
    height: i32,
    version: usize,
    backgrounds: Vec<Sprite>,
    key_count: u8,
    keys_collected: u8,
    paused: bool,
    pause_sprites: [Vec<(Rc<Texture2d>, [i32; 2], bool)>; 2],
    fade: [Rc<Texture2d>; 5],
    switch: i8
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
            TileType::Door(_, _) => false,
            TileType::KeyBackground(_) => false,
            TileType::Switch => false,
            TileType::Solid => true,
            TileType::SwitchBlock => true,
            TileType::Arrow(_) => false,
        }
    }
}

#[derive(Debug)]
enum TileType {
    Background,
    Solid,
    Door(String, [usize; 2]),
    KeyBackground([usize; 2]),
    Checkpoint,
    Switch,
    SwitchBlock,
    Arrow(Direction),
}

impl Default for TileType {
    fn default() -> Self {
        TileType::Background
    }
}

struct Entity {
    x: i32,
    y: i32,
    x_speed: f32,
    y_speed: f32,
    facing: bool,
    dead: bool,
    versions: [bool; 2],
    physics: bool,
    entity_type: EntityType,
}

enum EntityType {
    Player(Player),
    Enemy(Enemy),
    Key(Key),
}

struct Enemy {
    collision: bool,
    gravity: bool,
    deadly: bool,
    ai: AI,
    sprites: [Sprite; 2],
}

struct Key {
    collected: bool,
    sprite: Sprite,
    distance: i32,
}

enum AI {
    Pace(Direction),
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct Player {
    state: PlayerState,
    checkpoint_x: i32,
    checkpoint_y: i32,
    sprites: [PlayerSprites; 2],
}

struct PlayerSprites {
    walking: Sprite,
    standing: Sprite,
    falling: Sprite,
    jumping: Sprite,
    dying: Sprite,
    turning: Sprite,
    reviving: Sprite,
}

#[derive(Eq, PartialEq)]
enum PlayerState {
    Walking,
    Standing,
    Falling,
    Jumping,
    Dying,
    Turning(String),
    Reviving,
}

/*
use std::sync::{Mutex, Arc};
use std::io::{Read, Seek};

struct MixSource {
    tracks: [Vec<f32>; 2],
    balance: Arc<Mutex<f32>>,
    sample: usize,
    sample_count: usize,
    samples_rate: u32,
    channels: u16,
}


impl Source for MixSource {
    #[inline]
    fn get_current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn get_channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn get_samples_rate(&self) -> u32 {
        self.samples_rate
    }

    #[inline]
    fn get_total_duration(&self) -> Option<Duration> {
        let ms = self.sample_count * 1000 / (self.channels as usize * self.samples_rate as usize);
        Some(Duration::from_millis(ms as u64))
    }
}

impl<R> Iterator for MixSource<R> where R: Read + Seek {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.sample = (self.sample + 1) % self.sample_count;
    }
}*/

use std::io::Cursor;
use rodio::Source;

impl Game {
    pub fn load<F>(facade: &F) -> Game
        where F: glium::backend::Facade
    {
        let mut textures = HashMap::new();
        let mut levels = HashMap::new();
        let mut sounds = HashMap::new();
        for (name, content) in embed!("assets") {
            let name = String::from_utf8(name).unwrap().replace(r"\", "/");
            if name.ends_with(".gif") {
                textures.insert(name[..name.len() - 4].to_string(),
                                Sprite::load(facade, &content));
            } else if name.starts_with("levels/") {
                levels.insert(name[7..name.len() - 4].to_string(),
                              String::from_utf8(content).unwrap());
            } else if name.ends_with(".ogg") || name.ends_with(".wav") {
                sounds.insert(name[..name.len() - 4].to_string(), content);
            }
        }

        let mut font = HashMap::new();
        let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', '!', '?', ',', '~', '"', '\''];
        for i in 0..chars.len() {
            font.insert(chars[i],
                        Sprite::new(textures.get(&format!("GUI/font_{}", i)).unwrap()));
        }
        let (sender, reciever) = channel();

        /*let music = {
            let mut music = Vec::new();

            let mut channels = 0;
            let mut samples_rate = 0;
            let mut sample_count = 0;
            for unmixed in &[rodio::Decoder::new(Cursor::new(sounds.remove("sound/in life i choke").unwrap())),
                rodio::Decoder::new(Cursor::new(sounds.remove("sound/in death i breathe").unwrap()))] {
                channels = unmixed.get_channels();
                samples_rate = unmixed.get_samples_rate();
                music.push(unmixed.iter().collect());
            }
            let sample_count = music[0].len();
            MixSource {
                tracks: (),
                balance: (),
                sample: (),
                sample_count: sample_count,
                samples_rate: samples_rate,
                channels: channels
            }
        };

        let _ = std::thread::spawn(move || {
            let endpoint = rodio::get_default_endpoint().unwrap();
            let mut sink = rodio::Sink::new(&endpoint);
            sink.append(music);
            while let Ok(mix) = reciever.recv() {
            }
        });*/
        Game {
            palette_changed: true,
            textures: textures,
            levels: levels,
            state: GameState::Menu(Menu { selection: 0 }),
            palette_id: 0,
            palettes: [LIFE_PALETTE, DEATH_PALETTE],
            font: font,
            music: sender,
        }
    }

    fn load_level(textures: &HashMap<String, Texture>, string: &str) -> Level {
        let mut lines = string.lines();
        let mut tile_mappings = HashMap::new();
        let mut entity_mappings = HashMap::new();
        let mut sprite_map = HashMap::new();
        let mut tile_sprites = Vec::new();
        sprite_map.insert("tiles/blank", 0);
        tile_sprites.push(Sprite::new(textures.get("tiles/blank").unwrap()));
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
            let sprite_id = {
                    *sprite_map.entry(filename).or_insert({
                        if let Some(texture) = textures.get(filename) {
                            tile_sprites.push(Sprite::new(texture));
                            tile_sprites.len() - 1
                        } else {
                            0
                        }
                    })
                };
            let tile = Tile {
                sprite_id: sprite_id,
                tile_type: match values.next().unwrap().to_lowercase().as_ref() {
                    "background" => TileType::Background,
                    "solid" => TileType::Solid,
                    "door" => {
                        let next_level = values.next().unwrap().to_string();
                        let closed_id = {
                            let filename = values.next().unwrap_or("tiles/blank");
                            *sprite_map.entry(filename).or_insert({
                                if let Some(texture) = textures.get(filename) {
                                    tile_sprites.push(Sprite::new(texture));
                                    tile_sprites.len() - 1
                                } else {
                                    0
                                }
                            })
                        };
                        TileType::Door(next_level, [closed_id, sprite_id])},
                    "keybackground" => {
                        let closed_id = {
                            let filename = values.next().unwrap_or("tiles/blank");
                            *sprite_map.entry(filename).or_insert({
                                if let Some(texture) = textures.get(filename) {
                                    tile_sprites.push(Sprite::new(texture));
                                    tile_sprites.len() - 1
                                } else {
                                    0
                                }
                            })
                        };
                        TileType::KeyBackground([closed_id, sprite_id])},
                    "checkpoint" => TileType::Checkpoint,
                    "switch" => TileType::Switch,
                    "switchblock" => TileType::SwitchBlock,
                    "arrow" => TileType::Arrow(match values.next().unwrap().to_lowercase().as_ref() {
                                        "right" => Direction::Right,
                                        "up" => Direction::Up,
                                        "down" => Direction::Down,
                                        _ => Direction::Left,
                                    }),
                    _ => TileType::Background,
                },
            };
            tile_mappings.insert(character, Rc::new(tile));
        }
        while let Some(line) = lines.next() {
            if line == "LEVEL" {
                break;
            }
            let mut values = line.split(',');
            let character = values.next().unwrap().chars().next().unwrap();
            entity_mappings.insert(character, line);
        }
        let backgrounds = {
            let mut backgrounds = Vec::new();
            let prefix = lines.next().unwrap();
            let mut num = 0;
            while let Some(texture) = textures.get(&format!("{}_{}", prefix, num)) {
                backgrounds.push(Sprite::new(texture));
                num += 1;
            }
            backgrounds
        };
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
        let mut entities = Vec::new();
        entities.push(Entity {
            versions: [true, true],
            x: 0,
            y: 5 * 16,
            x_speed: 0.0,
            y_speed: 0.0,
            facing: true,
            dead: false,
            physics: true,
            entity_type: EntityType::Player(Player {
                checkpoint_x: 0,
                checkpoint_y: 5 * 16,
                state: PlayerState::Standing,
                sprites: [PlayerSprites {
                    walking: Sprite::new(&textures["player/MonsterWalk"]),
                    standing: Sprite::new(&textures["player/MonsterStand"]),
                    falling: Sprite::new(&textures["player/MonsterFalling"]),
                    jumping: Sprite::new(&textures["player/MonsterJump"]),
                    dying: Sprite::new(&textures["player/MonsterDeath"]),
                    turning: Sprite::new(&textures["player/MonsterTurn"]),
                    reviving: Sprite::new(&textures["tiles/blank"]),
                },
                    PlayerSprites {
                        walking: Sprite::new(&textures["player/HumanWalk"]),
                        standing: Sprite::new(&textures["player/HumanStand"]),
                        falling: Sprite::new(&textures["player/HumanFalling"]),
                        jumping: Sprite::new(&textures["player/HumanJump"]),
                        dying: Sprite::new(&textures["player/HumanDeath"]),
                        turning: Sprite::new(&textures["player/HumanTurn"]),
                        reviving: Sprite::new(&textures["player/HumanRevive"]),
                    }],
            }),
        });
        let mut key_count = 0;
        let mut tile_maps = Vec::new();
        for i in 0..2 {
            let mut tile_map = Vec::new();
            for y in 0..height {
                let mut chars = lines.next().unwrap_or("").chars();
                let mut row = Vec::new();
                for x in 0..width {
                    let character = chars.next().unwrap_or(' ');
                    row.push(tile_mappings.get(&character)
                        .unwrap_or(tile_mappings.get(&' ').unwrap())
                        .clone());
                    if let Some(entity_line) = entity_mappings.get(&character) {
                        let mut values = entity_line.split(',').skip(1);
                        let mut versions = [false, false];
                        versions[i] = true;
                        let entity = match values.next().unwrap().to_lowercase().as_ref() {
                            "key" => {
                                key_count += 1;
                                Some(Entity {
                                    x: (width - 1 - x) * 16,
                                    y: (height - 1 - y) * 16,
                                    x_speed: 0.0,
                                    y_speed: 0.0,
                                    facing: false,
                                    dead: false,
                                    versions: versions,
                                    physics: false,
                                    entity_type: EntityType::Key(Key {
                                        collected: false,
                                        sprite: Sprite::new(&textures["entities/Key"]),
                                        distance: 0,
                                    }),
                                })
                            },
                            "enemy" => {
                                let life_filename = values.next().unwrap();
                                let death_filename = values.next().unwrap();
                                let mut facing = false;
                                let ai = match values.next().unwrap() {
                                    _ => AI::Pace(match values.next().unwrap().to_lowercase().as_ref() {
                                        "right" => {facing = true; Direction::Right},
                                        "up" => Direction::Up,
                                        "down" => Direction::Down,
                                        _ => Direction::Left,
                                    })
                                };
                                let mut deadly = true;
                                let mut gravity = true;
                                let mut collision = true;
                                while let Some(arg) = values.next() {
                                    match arg.to_lowercase().as_ref() {
                                        "safe" => {deadly = false;},
                                        "float" => {gravity = false;},
                                        "noclip" => {collision = false;},
                                        _ => (),
                                    }
                                }
                                Some(Entity {
                                    x: (width - 1 - x) * 16,
                                    y: (height - 1 - y) * 16,
                                    x_speed: 0.0,
                                    y_speed: 0.0,
                                    facing: facing,
                                    dead: false,
                                    versions: versions,
                                    physics: gravity && collision,
                                    entity_type: EntityType::Enemy(Enemy {
                                        collision: collision,
                                        gravity: gravity,
                                        deadly: deadly,
                                        ai: ai,
                                        sprites: [Sprite::new(textures.get(life_filename).unwrap()), Sprite::new(textures.get(death_filename).unwrap())],
                                    }),
                                })
                            },
                            _ => None,
                        };
                        entities.push(entity.unwrap());
                    }
                }
                tile_map.push(row);
            }
            tile_map.reverse();
            tile_maps.push(tile_map);
        }
        let fade = {
            let texture = &textures["Fade"];
            [
                texture[0].0.clone(),
                texture[1].0.clone(),
                texture[2].0.clone(),
                texture[3].0.clone(),
                texture[4].0.clone(),
            ]
        };
        Level {
            tile_map: [tile_maps.remove(0), tile_maps.remove(0)],
            tile_sprites: tile_sprites,
            width: width,
            height: height,
            wraparound: wraparound,
            entities: entities,
            version: 0,
            backgrounds: backgrounds,
            key_count: key_count,
            keys_collected: 0,
            paused: false,
            pause_sprites: [Vec::new(), Vec::new()],
            fade: fade,
            switch: 0
        }
    }

    fn text(&mut self, text: &str, mut x: i32, mut y: i32, width: i32) -> Vec<(Rc<Texture2d>, [i32; 2], bool)> {
        let start = x;
        let mut vec = Vec::new();
        for word in text.split(' ') {
            if x + word.len() as i32 * 6 > start + width && word.len() as i32 * 6 < width {
                x = start;
                y -= 10
            } else if x > start {
                x += 6
            }
            for c in word.to_uppercase().chars() {
                if self.font.contains_key(&c) {
                    vec.push((self.font.get_mut(&c).unwrap().texture(), [x, y], false));
                }
                x += 6;
                if x > start + width {
                    x = start;
                    y -= 10;
                }
            }
        }
        vec
    }

    pub fn step(&mut self, input: &Input) -> Vec<(Rc<Texture2d>, [i32; 2], bool)> {
        let mut sprites = Vec::new();
        let mut new_level = Option::None;
        if let Some(new_state) = match self.state {
            GameState::Menu(ref mut menu) => {
                Some(GameState::Level(Game::load_level(&self.textures,
                                                       &self.levels.get("Tutorial_Level").unwrap())))
            }
            GameState::Level(ref mut level) => {
                self.palette_id = level.version;
                if input.start {
                    level.paused = !level.paused;
                }
                let camera_x = if level.entities[0].x > 80 {
                    if level.entities[0].x < level.width as i32 * 16 - 80 {
                        level.entities[0].x - 80
                    } else {
                        level.width as i32 * 16 - 160
                    }
                } else {
                    0
                };
                let camera_y = if level.entities[0].y > 72 {
                    if level.entities[0].y < level.height as i32 * 16 - 72 {
                        level.entities[0].y - 72
                    } else {
                        level.height as i32 * 16 - 144
                    }
                } else {
                    0
                };
                self.palette_changed = false;
                for i in 0..level.backgrounds.len() {
                    let offset = (0 - (i * i) as i32 * camera_x / (level.backgrounds.len() * level.backgrounds.len()) as i32) % 160;
                    sprites.push((level.backgrounds[i].texture(), [offset, 0], false));
                    sprites.push((level.backgrounds[i].texture(), [offset + 160, 0], false));
                }
                let mut fade_index = 0;
                if level.switch > 0 {
                    level.paused = true;
                    if level.switch >= 57 {
                        self.palette_id = 2;
                    } else if level.switch >= 48 {
                        self.palette_id = 3;
                    } else if level.switch >= 18 {
                        self.palette_id = 4;
                        fade_index = (level.switch - 18) / 6;
                    } else if level.switch >= 9 {
                        if level.switch == 17 {
                            let mut player_entity = &mut level.entities[0];
                            player_entity.dead = false;
                            if let EntityType::Player(ref mut player) = player_entity.entity_type {
                                player.state = PlayerState::Standing;
                                if level.version == 0 {
                                    while Level::get_tile(&level.tile_map[1],
                                                          player_entity.x,
                                                          player_entity.y,
                                                          level.wraparound)
                                        .is_solid() ||
                                        Level::get_tile(&level.tile_map[1],
                                                        player_entity.x + 15,
                                                        player_entity.y,
                                                        level.wraparound)
                                            .is_solid() {
                                        player_entity.y += 16;
                                    }
                                    level.version = 1;
                                } else {
                                    player_entity.x = player.checkpoint_x;
                                    player_entity.y = player.checkpoint_y;
                                    level.version = 0;
                                }
                            }
                        }
                        self.palette_id = 5;
                    } else {
                        self.palette_id = 6;
                    }
                    level.switch -= 1;
                    //0, 0 > 0.0
                    //60, 1 > 1.0
                    //0, 1 > 1.0
                    //60, 1 > 0.0
                    let mix = if (level.switch > 17) ^ (level.version == 0) {(60 - level.switch) as f32 / 60.0} else { level.switch as f32 / 60.0 };
                    //self.music.send(mix).unwrap();
                    if level.switch == 0 {
                        level.paused = false;
                    }
                    if level.version == 0 {
                        fade_index = 4 - fade_index;
                        self.palette_id = 6 - (self.palette_id - 2);
                    }
                }
                let mut relative_sprites = Vec::new();
                if !level.paused {
                    let textures: Vec<Rc<Texture2d>> = level.tile_sprites.iter_mut().map(|sprite| sprite.texture()).collect();
                    for y in 0..level.height as usize {
                        for x in 0..level.width as usize {
                            let tile = &level.tile_map[level.version][y][x];
                            let mut sprite_id = tile.sprite_id;
                            if let TileType::Door(_, sprites) = tile.tile_type {
                                sprite_id = sprites[(level.keys_collected >= level.key_count) as usize];
                            }
                            if let TileType::KeyBackground(sprites) = tile.tile_type {
                                sprite_id = sprites[(level.keys_collected >= level.key_count) as usize];
                            }
                            if tile.sprite_id != 0 {
                                relative_sprites.push((textures[sprite_id].clone(), [x as i32 * 16, y as i32 * 16], false))
                            }
                        }
                    }
                    let mut player_x = 0;
                    let mut player_y = 0;
                    let mut player_dead = false;
                    for entity in level.entities.iter_mut() {
                        if !entity.versions[level.version] {
                            continue;
                        }
                        let grounded = Level::get_tile(&level.tile_map[level.version],
                                                       entity.x,
                                                       entity.y - 1,
                                                       level.wraparound)
                            .is_solid() ||
                            Level::get_tile(&level.tile_map[level.version],
                                            entity.x + 15,
                                            entity.y - 1,
                                            level.wraparound)
                                .is_solid();
                        let mut collisions = Vec::new();
                        if entity.physics && !entity.dead {
                            entity.x_speed *= 0.75;
                            if entity.x_speed > 0.0 {
                                if entity.x_speed < 1.0 {
                                    entity.x_speed = 0.0;
                                } else {
                                    let target = entity.x + entity.x_speed as i32 + 16;
                                    if target >= level.width as i32 * 16 ||
                                        Level::get_tile(&level.tile_map[level.version],
                                                        target,
                                                        entity.y,
                                                        level.wraparound)
                                            .is_solid() ||
                                        Level::get_tile(&level.tile_map[level.version],
                                                        target,
                                                        entity.y + 15,
                                                        level.wraparound)
                                            .is_solid() {
                                        entity.x = (entity.x + entity.x_speed as i32) / 16 * 16;
                                        entity.x_speed = 0.0;
                                        collisions.push(Direction::Right);
                                    } else {
                                        entity.x += entity.x_speed as i32;
                                    }
                                }
                            } else if entity.x_speed < 0.0 {
                                if entity.x_speed > -1.0 {
                                    entity.x_speed = 0.0
                                } else {
                                    let target = entity.x + entity.x_speed as i32;
                                    if target < 0 ||
                                        Level::get_tile(&level.tile_map[level.version],
                                                        target,
                                                        entity.y,
                                                        level.wraparound)
                                            .is_solid() ||
                                        Level::get_tile(&level.tile_map[level.version],
                                                        target,
                                                        entity.y + 15,
                                                        level.wraparound)
                                            .is_solid() {
                                        entity.x = (entity.x + entity.x_speed as i32 + 16) / 16 * 16;
                                        entity.x_speed = 0.0;
                                        collisions.push(Direction::Left);
                                    } else {
                                        entity.x += entity.x_speed as i32;
                                    }
                                }
                            }
                            if !grounded {
                                entity.y_speed -= 0.18;
                            }
                            if entity.y_speed > 0.0 {
                                let target = entity.y + entity.y_speed as i32 + 16;
                                if (!level.wraparound && target >= level.height as i32 * 16) ||
                                    Level::get_tile(&level.tile_map[level.version],
                                                    entity.x,
                                                    target,
                                                    level.wraparound)
                                        .is_solid() ||
                                    Level::get_tile(&level.tile_map[level.version],
                                                    entity.x + 15,
                                                    target,
                                                    level.wraparound)
                                        .is_solid() {
                                    entity.y = (entity.y + entity.y_speed as i32) / 16 * 16;
                                    entity.y_speed = 0.0;
                                    collisions.push(Direction::Up);
                                } else {
                                    entity.y += entity.y_speed as i32;
                                }
                            } else if entity.y_speed < 0.0 {
                                if grounded {
                                    entity.y_speed = 0.0;
                                }
                                if entity.y_speed < -2.0 {
                                    entity.y_speed = -2.0
                                }
                                let target = entity.y + entity.y_speed as i32;
                                if (!level.wraparound && target < 0) ||
                                    Level::get_tile(&level.tile_map[level.version],
                                                    entity.x,
                                                    target,
                                                    level.wraparound)
                                        .is_solid() ||
                                    Level::get_tile(&level.tile_map[level.version],
                                                    entity.x + 15,
                                                    target,
                                                    level.wraparound)
                                        .is_solid() {
                                    entity.y = (entity.y + entity.y_speed as i32 + 16) / 16 * 16;
                                    entity.y_speed = 0.0;
                                } else {
                                    entity.y += entity.y_speed as i32;
                                }
                            }
                        }
                        if level.wraparound {
                            entity.y = (entity.y + 16 * level.height as i32) % (16 * level.height as i32)
                        }
                        if let Some(sprite) = match entity.entity_type {
                            EntityType::Player(ref mut player) => {
                                player_x = entity.x;
                                player_y = entity.y;
                                match player.state {
                                    PlayerState::Dying => {
                                        if let Some(ref mut animator) = player.sprites[level.version].dying.animator {
                                            if animator.instant.elapsed() > player.sprites[level.version].dying.texture[animator.index].1 {
                                                animator.instant = Instant::now();
                                                animator.index += 1;
                                            }
                                            if animator.index >= player.sprites[level.version].dying.texture.len() {
                                                animator.index = 0;
                                                level.switch = 60;
                                                entity.dead = false;
                                                level.version = level.version ^ 1;
                                            }
                                        }
                                    },
                                    PlayerState::Turning(ref level_name) => {
                                        if let Some(ref mut animator) = player.sprites[level.version].turning.animator {
                                            if animator.instant.elapsed() > player.sprites[level.version].turning.texture[animator.index].1 {
                                                animator.instant = Instant::now();
                                                animator.index += 1;
                                            }
                                            if animator.index >= player.sprites[level.version].turning.texture.len() {
                                                new_level = Some(level_name.clone());
                                                break;
                                            }
                                        }
                                    },
                                    PlayerState::Reviving => {
                                        if let Some(ref mut animator) = player.sprites[level.version].reviving.animator {
                                            if animator.instant.elapsed() > player.sprites[level.version].reviving.texture[animator.index].1 {
                                                animator.instant = Instant::now();
                                                animator.index += 1;
                                            }
                                            if animator.index >= player.sprites[level.version].reviving.texture.len() {
                                                level.switch = 60;
                                                level.version = level.version ^ 1;
                                            }
                                        }
                                    },
                                    _ => {
                                        entity.dead = entity.dead || (!level.wraparound && entity.y <= 0);
                                        if entity.dead {
                                            player.state = PlayerState::Dying;
                                            player.sprites[level.version].dying.reset();
                                        } else {
                                            player.state = PlayerState::Standing;
                                            if input.left {
                                                player.state = PlayerState::Walking;
                                                entity.facing = false;
                                                entity.x_speed = -2.0;
                                            } else if input.right {
                                                player.state = PlayerState::Walking;
                                                entity.facing = true;
                                                entity.x_speed = 2.0;
                                            }
                                            if input.a && grounded {
                                                player.state = PlayerState::Jumping;
                                                entity.y_speed = 4.0;
                                            }
                                            if entity.y_speed < 0.0 {
                                                player.state = PlayerState::Falling;
                                                player.sprites[level.version].falling.reset();
                                            }
                                            if input.b {
                                                match Level::get_tile(&level.tile_map[level.version],
                                                                      entity.x + 8,
                                                                      entity.y + 8,
                                                                      level.wraparound)
                                                    .tile_type {
                                                    TileType::Checkpoint => {
                                                        player.checkpoint_x = entity.x;
                                                        player.checkpoint_y = entity.y;
                                                        if level.version == 1 {
                                                            player.state = PlayerState::Reviving;
                                                            player.sprites[level.version].reviving.reset();
                                                        }
                                                    }
                                                    TileType::Door(ref level_name, _) => {
                                                        if level.keys_collected >= level.key_count {
                                                            player.state = PlayerState::Turning(level_name.clone());
                                                            player.sprites[level.version].turning.reset();
                                                        }
                                                    }
                                                    _ => (),
                                                }
                                            }
                                        }
                                    }
                                }
                                Option::Some(match player.state {
                                    PlayerState::Falling => player.sprites[level.version].falling.texture(),
                                    PlayerState::Standing => player.sprites[level.version].standing.texture(),
                                    PlayerState::Jumping => player.sprites[level.version].jumping.texture(),
                                    PlayerState::Walking => player.sprites[level.version].walking.texture(),
                                    PlayerState::Dying => {
                                        let mut index = 0;
                                        if let Some(ref mut animator) = player.sprites[level.version].dying.animator {
                                            index = animator.index;
                                        }
                                        player.sprites[level.version].dying.texture[index].0.clone()
                                    },
                                    PlayerState::Reviving => {
                                        let mut index = 0;
                                        if let Some(ref mut animator) = player.sprites[level.version].reviving.animator {
                                            index = animator.index;
                                        }
                                        player.sprites[level.version].reviving.texture[index].0.clone()
                                    },
                                    PlayerState::Turning(_) => {
                                        let mut index = 0;
                                        if let Some(ref mut animator) = player.sprites[level.version].turning.animator {
                                            index = animator.index;
                                        }
                                        player.sprites[level.version].turning.texture[index].0.clone()
                                    },
                                })
                            }
                            EntityType::Enemy(ref mut enemy) => {
                                if enemy.deadly {
                                    let x_distance = player_x - entity.x;
                                    let y_distance = player_y - entity.y;
                                    if x_distance * x_distance + y_distance * y_distance < 32 * 32 {
                                        if player_x + 15 >= entity.x && player_x <= entity.x + 15 && player_y + 15 >= entity.y && player_y <= entity.y + 15 {
                                            player_dead = true;
                                        }
                                    }
                                }
                                let AI::Pace(ref mut direction) = enemy.ai;
                                {
                                    if *direction == Direction::Left {
                                        entity.facing = false;
                                    } else if *direction == Direction::Right {
                                        entity.facing = true;
                                    }
                                    if !enemy.collision {
                                        match *direction {
                                            Direction::Down => { entity.y -= 1 },
                                            Direction::Up => { entity.y += 1 },
                                            Direction::Left => { entity.x -= 1 },
                                            Direction::Right => { entity.x += 1 },
                                        }
                                    } else {
                                        for collision in collisions {
                                            if *direction == collision {
                                                match collision {
                                                    Direction::Down => {*direction = Direction::Up},
                                                    Direction::Up => {*direction = Direction::Down},
                                                    Direction::Left => {*direction = Direction::Right},
                                                    Direction::Right => {*direction = Direction::Left},
                                                }
                                            }
                                        }
                                        match *direction {
                                            Direction::Down => { entity.y_speed = -2.0 },
                                            Direction::Up => { entity.y_speed = 2.0 },
                                            Direction::Left => { entity.x_speed = -2.0 },
                                            Direction::Right => { entity.x_speed = 2.0 },
                                        }
                                    }
                                    if let TileType::Arrow(ref arrow_dir) =  Level::get_tile(&level.tile_map[level.version],
                                                          entity.x + 8,
                                                          entity.y + 8,
                                                          level.wraparound).tile_type {
                                        *direction = arrow_dir.clone();
                                    }
                                }
                                Some(enemy.sprites[level.version].texture())
                            },
                            EntityType::Key(ref mut key) => {
                                let x_distance = player_x - entity.x;
                                let y_distance = player_y - entity.y;
                                if !key.collected {
                                    if x_distance * x_distance + y_distance * y_distance < 32 * 32 {
                                        if player_x + 15 >= entity.x && player_x <= entity.x + 15 && player_y + 15 >= entity.y && player_y <= entity.y + 15 {
                                            key.collected = true;
                                            level.keys_collected += 1;
                                            key.distance = level.keys_collected as i32 * 12;
                                            entity.versions = [true, true];
                                        }
                                    }
                                }
                                if key.collected {
                                    if x_distance > key.distance || x_distance < key.distance {
                                        entity.x += x_distance / key.distance;
                                    }
                                    if y_distance > key.distance || y_distance < key.distance {
                                        entity.y += y_distance / key.distance;
                                    }
                                }
                                Some(key.sprite.texture())
                            }
                        } {
                            relative_sprites.push((sprite.clone(), [entity.x, entity.y], entity.facing));
                            if level.wraparound && entity.y > (level.height as i32 * 16 - 16) {
                                relative_sprites.push((sprite, [entity.x, entity.y - (level.height as i32 * 16)], entity.facing));
                            }
                        }
                    }
                    if player_dead {
                        level.entities[0].dead = true;
                    }
                }
                if !level.paused {
                    level.pause_sprites[level.version] = relative_sprites;
                }
                if level.switch == 60 {
                    level.version = level.version ^ 1;
                }
                for &(ref sprite, position, flip) in &level.pause_sprites[level.version & 1]{
                    sprites.push((sprite.clone(), [position[0] - camera_x, position[1] - camera_y], flip));
                }
                if self.palette_id == 4 {
                    sprites.push((level.fade[fade_index as usize].clone(), [0, 0], false));
                }
                None
            }
        } {
            self.state = new_state;
        }
        if let Some(level_name) = new_level {
            //self.music.send(0.0).unwrap();
            self.state = GameState::Level(Game::load_level(&self.textures, &self.levels.get(&level_name).unwrap()));
        }
        //        let mut text = self.text("Okay it works, cool", 20, 130, 116);
        // sprites.append(&mut text);
        sprites
    }
}

impl Level {
    pub fn get_tile(tile_map: &TileMap, x: i32, mut y: i32, wraparound: bool) -> Rc<Tile> {
        let height = tile_map.len();
        let width = tile_map[0].len();
        if wraparound {
            y = (y + height as i32 * 16) % (height as i32 * 16);
        }
        if x < 0 || x >= width as i32 * 16 || y < 0 || y >= height as i32 * 16 {
            Rc::new(Tile {
                sprite_id: 0,
                tile_type: TileType::Background,
            })
        } else {
            tile_map[(y / 16) as usize][(x / 16) as usize].clone()
        }
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

        let mut texture: Vec<(Rc<Texture2d>, Duration)> = Vec::new();

        while let Some(frame) = decoder.read_next_frame().unwrap() {
            texture.push((Rc::new(Texture2d::new(facade,
                                                 glium::texture::RawImage2d {
                                                     data: frame.buffer.clone(),
                                                     width: frame.width as u32,
                                                     height: frame.height as u32,
                                                     format: glium::texture::ClientFormat::U8,
                                                 })
                .unwrap()),
                          Duration::from_millis(10 * frame.delay as u64)));
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
