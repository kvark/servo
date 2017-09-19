/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::Key;
use dom::bindings::reflector::Reflector;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuDescriptor {
    reflector_: Reflector,
    id: Key,
}

impl WebGpuDescriptor {
    pub fn new_inherited(id: Key) -> Self {
        WebGpuDescriptor {
            reflector_: Reflector::new(),
            id,
        }
    }

    pub fn get_id<T: From<Key>>(&self) -> T {
        self.id.into()
    }
}
