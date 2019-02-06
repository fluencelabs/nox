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

#[derive(Debug)]
pub struct StepCounter {
    steps: u8,
    current_step: u8,
}

impl StepCounter {
    pub fn new() -> StepCounter {
        StepCounter {
            steps: 0,
            current_step: 0,
        }
    }

    // adds one step
    pub fn register(&mut self) {
        self.steps = self.steps + 1;
    }

    // gets formatted next step
    pub fn format_next_step(&mut self) -> String {
        let step = format!("{}/{}", self.current_step, self.steps);
        self.current_step = self.current_step + 1;
        step
    }
}
