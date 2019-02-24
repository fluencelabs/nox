/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::game::Game;
use std::{cell::RefCell, rc::Weak};

/// Represents player with name and a link to Game.
pub struct Player {
    pub name: String,
    pub game: Weak<RefCell<Game>>,
}

impl Player {
    pub fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Player {
            name: name.into(),
            game: Weak::new(),
        }
    }
}
