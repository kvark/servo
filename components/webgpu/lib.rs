/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub extern crate gfx_corell as gpu;
pub extern crate gfx_device_vulkanll as backend;
extern crate heapsize;
#[macro_use]
extern crate heapsize_derive;
#[macro_use]
extern crate serde_derive;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, HeapSizeOf)]
pub enum QueueType {
    Graphics,
    Compute,
    Transfer,
}
