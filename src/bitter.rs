mod entities;
mod events; 

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use slotmap::{new_key_type, SlotMap, SecondaryMap};

use entities::{
    DeathMarker,
    Farm,
    GameEntity,
    Villager,
};
use events::{WE,WorldEvent};

pub const GRID_WIDTH: u8 = 8;
pub const GRID_HEIGHT: u8 = 8;

new_key_type! { pub struct EntityKey; }

pub type EntityId = usize;
pub type Ticks = usize;

#[derive(Copy,Clone,Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

type Dir = Direction;

const CARDINAL_DIRECTIONS: [Direction; 4] = [
    Dir::Up,
    Dir::Down,
    Dir::Left,
    Dir::Right,
];

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0, 4) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

pub type Coords = (u8, u8);

pub struct World {
    events: Vec<WorldEvent>,
    entities: SlotMap<EntityKey, GameEntity>,
    pub coords: SecondaryMap<EntityKey, Coords>,
    last_id: EntityId,
    pub death_markers: SecondaryMap<EntityKey, DeathMarker>,
    pub farms: SecondaryMap<EntityKey, Farm>,
    ticks: Ticks,
    pub satiation: SecondaryMap<EntityKey, u8>,
    pub villagers: SecondaryMap<EntityKey, Villager>,
}

impl World {
    pub fn new() -> Self {
        let mut world = World {
            entities: SlotMap::with_key(),
            coords: SecondaryMap::new(),
            events: vec![],
            last_id: 0,
            ticks: 0,
            death_markers: SecondaryMap::new(),
            farms: SecondaryMap::new(),
            satiation: SecondaryMap::new(),
            villagers: SecondaryMap::new(),
        };

        world.add_villager_at(4, 4);

        world.add_farm_at(5, 4);
        world.add_farm_at(5, 5);
        world.add_farm_at(5, 6);

        world
    }

    fn process_events(&mut self) {
        while let Some(evt) = self.events.pop() {
            let mut new_events: Vec<WorldEvent> = vec![];

            match evt {
                WE::VillagerMoved(key, dir) => self.villager_moved(key, dir),
                WE::VillagerAte(key) => self.villager_ate(key),
                WE::VillagerHungered(key) => self.villager_hungered(key, &mut new_events),
                WE::FarmGrew(key) => self.farm_grew(key, &mut new_events),
                WE::FarmHarvested(key) => self.farm_harvested(key),
                WE::VillagerDied(vk) => self.villager_died(vk),
                WE::FarmAdded(coords) => self.farm_added(coords),
                WE::VillagerHarvested(vk) => self.villager_harvested(vk, &mut new_events),
                WE::GravesCleared => self.graves_cleared(),
                WE::FarmsCultivated => self.farms_cultivated(&mut new_events),
                WE::VillagersFarmed => self.villagers_farmed(&mut new_events),
                WE::VillagersMoved => self.villagers_moved(&mut new_events),
            }

            self.events.extend(new_events);
        }
    }

    fn villager_moved(&mut self, key: EntityKey, dir: Direction) {
        let c = self.coords[key];
        self.coords[key] = coords_after_move(c, dir);
    }

    fn villager_ate(&mut self, key: EntityKey) {
        self.satiation[key] += 1;

        let mut villager = self.villagers[key];
        villager.last_ate = self.ticks;
        self.villagers[key] = villager;
    }

    fn villager_hungered(&mut self, key: EntityKey, new_events: &mut Vec<WorldEvent>) {
        if self.satiation[key] > 0 {
            self.satiation[key] -= 1;

            let mut villager = self.villagers[key];
            villager.last_ate = self.ticks;
            self.villagers[key] = villager;
        } else {
            new_events.push(WE::VillagerDied(key));
        }
    }

    fn farm_grew(&mut self, key: EntityKey, new_events: &mut Vec<WorldEvent>) {
        let mut farm = self.farms[key];

        if self.ticks - farm.last_grew < 20 {
            return;
        }

        farm.last_grew = self.ticks;
        self.farms[key] = farm;

        let farm_coords = self.coords[key];

        let mut all_possible_coords: Vec<Coords> = vec![];

        for dir in CARDINAL_DIRECTIONS.iter() {
            if can_move_in_dir(farm_coords, *dir) {
                let new_coords = coords_after_move(farm_coords, *dir);
                all_possible_coords.push(new_coords);
            }
        }

        for key in self.farms.keys() {
            if all_possible_coords.is_empty() {
                continue;
            }

            let occupied_coords = self.coords[key];

            all_possible_coords.retain(|c| *c != occupied_coords);

        }

        if all_possible_coords.len() == 0 {
            return;
        }

        let mut rng = rand::thread_rng();

        let ci = rng.gen_range(0, all_possible_coords.len());

        let new_coords = all_possible_coords[ci];

        new_events.push(WE::FarmAdded(new_coords));
    }

    fn farm_harvested(&mut self, key: EntityKey) {
        self.farms.remove(key);
    }

    fn villager_died(&mut self, vk: EntityKey) {
        let coords = self.coords[vk];

        let dmk = self.entities.insert(GameEntity);

        let dm = DeathMarker { key: dmk };

        self.death_markers.insert(dmk, dm);
        self.coords.insert(dmk, coords);

        self.villagers.remove(vk);
    }

    fn farm_added(&mut self, coords: Coords) {
        let (x, y) = (coords.0, coords.1);

        let new_id = self.last_id + 1;

        let entity = GameEntity;
        let key = self.entities.insert(entity);

        let farm = Farm::new(new_id, key, x, y, self.ticks);

        self.last_id = new_id;

        self.farms.insert(key, farm);
        self.coords.insert(key, (x, y));

        self.last_id = new_id;
    }

    fn villager_harvested(&mut self, vk: EntityKey, new_events: &mut Vec<WorldEvent>) {
        let mut rng = rand::thread_rng();

        let villager = self.villagers[vk];
        let satiation = self.satiation[vk];

        let mut unharvested_farms: Vec<&Farm> = self.farms.values().collect();

        let time_since_last_ate = self.ticks - villager.last_ate;
        let need_to_eat = satiation < 5 && time_since_last_ate < 40;
        let food_left_to_eat = unharvested_farms.len() > 0;

        if need_to_eat {
            if food_left_to_eat {
                let farm_to_eat_index = rng.gen_range(0, unharvested_farms.len());
                let farm = unharvested_farms.remove(farm_to_eat_index);

                new_events.push(WE::FarmHarvested(farm.key));
                new_events.push(WE::VillagerAte(vk));
            } else {
                new_events.push(WE::VillagerHungered(vk));
            }
        }
    }

    fn graves_cleared(&mut self) {
        self.death_markers.clear();
    }

    fn farms_cultivated(&mut self, new_events: &mut Vec<WorldEvent>) {
        for farm in self.farms.values() {
            new_events.push(WE::FarmGrew(farm.key));
        }
    }

    fn villagers_farmed(&mut self, new_events: &mut Vec<WorldEvent>) {
        for vk in self.villagers.keys() {
            new_events.push(WE::VillagerHarvested(vk));
        }
    }

    fn villagers_moved(&mut self, new_events: &mut Vec<WorldEvent>) {
        for key in self.villagers.keys() {
            let direction: Direction = rand::random();

            new_events.push(WE::VillagerMoved(key, direction));
        }
    }

    pub fn add_villager_at(&mut self, x: u8, y: u8) -> EntityId {
        let new_id = self.last_id + 1;

        let entity = GameEntity;
        let key = self.entities.insert(entity);

        let villager = Villager::new(new_id, key, self.ticks);

        self.villagers.insert(key, villager);
        self.coords.insert(key, (x, y));
        self.satiation.insert(key, 1);

        self.last_id = new_id;

        new_id
    }

    pub fn add_farm_at(&mut self, x: u8, y: u8) -> EntityId {
        let new_id = self.last_id + 1;

        let entity = GameEntity;
        let key = self.entities.insert(entity);

        let farm = Farm::new(new_id, key, x, y, self.ticks);

        self.last_id = new_id;

        self.farms.insert(key, farm);
        self.coords.insert(key, (x, y));

        self.last_id = new_id;

        new_id
    }

    pub fn tick(&mut self) {
        self.ticks += 1;

        if (self.ticks + 1) % 20 == 0 {
            self.advance_world();
        }
    }

    fn advance_world(&mut self) {
        // self.events is a LIFO stack
        self.events.push(WE::VillagersMoved);
        self.events.push(WE::VillagersFarmed);
        self.events.push(WE::FarmsCultivated);
        self.events.push(WE::GravesCleared);

        self.process_events();
    }

    pub fn villager_id_at(&self, x: u8, y: u8) -> Option<EntityId> {
        for v in self.villagers.values() {
            let (vx, vy) = self.coords[v.key];
            if vx == x && vy == y {
                return Some(v.id);
            }
        }

        None
    }

    pub fn villager(&self, id: EntityId) -> Option<Villager> {
        for v in self.villagers.values() {
            if v.id == id {
                return Some(v.clone());
            }
        }

        None
    }
}

fn coords_after_move(coords: Coords, dir: Direction) -> Coords {
    let (mut x, mut y) = (coords.0, coords.1);

    // Remember to account for the border
    match dir {
        Direction::Up => {
            if y > 1 {
                y -= 1;
            }
        },
        Direction::Down => {
            if y < GRID_HEIGHT - 2 {
                y += 1;
            }
        },
        Direction::Left => {
            if x > 1 {
                x -= 1;
            }
        },
        Direction::Right => {
            if x < GRID_WIDTH - 2 {
                x += 1;
            }
        },
    }

    (x, y)
}

fn can_move_in_dir(coords: Coords, dir: Direction) -> bool {
    let (x, y) = (coords.0, coords.1);

    // Remember to account for the border
    match dir {
        Dir::Up => y > 1,
        Dir::Down => y < GRID_HEIGHT - 2,
        Dir::Left => x > 1,
        Dir::Right => x < GRID_WIDTH - 2,
    }
}
