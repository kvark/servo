/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::WebMetalEncoderCommand;
use dom::bindings::codegen::Bindings::WebMetalRenderEncoderBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::num::Finite;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use ipc_channel::ipc::IpcSender;

#[dom_struct]
pub struct WebMetalRenderEncoder {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_sender: IpcSender<WebMetalEncoderCommand>,
}

impl WebMetalRenderEncoder {
    pub fn new(global: &GlobalScope,
               ipc_sender: IpcSender<WebMetalEncoderCommand>)
               -> Root<WebMetalRenderEncoder> {
        let object = box WebMetalRenderEncoder {
            reflector: Reflector::new(),
            ipc_sender: ipc_sender,
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalRenderEncoderMethods for WebMetalRenderEncoder {
    fn ClearColor(&self, _r: Finite<f32>, _g: Finite<f32>, _b: Finite<f32>, _a: Finite<f32>) {
        //TODO
    }
    fn EndEncoding(&self) {
        //TODO
    }
}
