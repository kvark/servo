/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUTextureBinding as binding;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::root::DomRoot;
use dom::window::Window;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGPUTexture {
    reflector_: Reflector,
    id: w::TextureId,
}

impl WebGPUTexture {
    #[allow(unrooted_must_root)]
    pub fn new(window: &Window, info: w::TextureInfo) -> DomRoot<Self> {
        let object = WebGPUTexture {
            reflector_: Reflector::new(),
            id: info.id,
        };
        reflect_dom_object(Box::new(object), window, binding::Wrap)
    }
}

impl Drop for WebGPUTexture {
    fn drop(&mut self) {
        //TODO
    }
}
