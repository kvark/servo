/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::webmetalcommandqueue::WebMetalCommandQueue;

#[dom_struct]
pub struct WebMetalDevice {
    reflector: Reflector,
}

impl binding::WebMetalDeviceMethods for WebMetalDevice {
    fn GetCommandQueue(&self) -> Root<WebMetalCommandQueue> {
        WebMetalCommandQueue::new(&self.global())
    }
}
