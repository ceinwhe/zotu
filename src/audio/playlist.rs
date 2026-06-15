use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{collections::HashMap, sync::Arc};

use crate::db::AlbumInfo;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum LoopMode {
    Random,
    Single,
    List,
}

impl LoopMode {
    pub fn next(&self) -> Self {
        match self {
            LoopMode::List => LoopMode::Single,
            LoopMode::Single => LoopMode::Random,
            LoopMode::Random => LoopMode::List,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayState {
    Play,
    Paused,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct PlayProgress {
    pub elapsed: u64,
    pub duration: u64,
    pub progress: f32,
}

pub struct PlayList {
    pub items: Arc<Vec<AlbumInfo>>,
    pub index: HashMap<Uuid, usize>,
    pub shuffle_order: Vec<usize>,
}

impl PlayList {
    pub fn new(items: Arc<Vec<AlbumInfo>>) -> Self {
        let index = items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id(), i))
            .collect();
        let shuffle_order = (0..items.len()).collect();
        Self {
            items,
            index,
            shuffle_order,
        }
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.shuffle_order.shuffle(&mut rng);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, index: usize) -> Option<&AlbumInfo> {
        self.items.get(index)
    }

    pub fn index_of(&self, id: &Uuid) -> Option<usize> {
        self.index.get(id).copied()
    }
}
