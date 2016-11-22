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
use std::cell::Cell;

#[dom_struct]
pub struct WebMetalRenderEncoder {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_sender: IpcSender<WebMetalEncoderCommand>,
    is_open: Cell<bool>,
}

impl WebMetalRenderEncoder {
    pub fn new(global: &GlobalScope,
               ipc_sender: IpcSender<WebMetalEncoderCommand>)
               -> Root<WebMetalRenderEncoder> {
        let object = box WebMetalRenderEncoder {
            reflector: Reflector::new(),
            ipc_sender: ipc_sender,
            is_open: Cell::new(true),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalRenderEncoderMethods for WebMetalRenderEncoder {
    fn ClearColor(&self, r: Finite<f32>, g: Finite<f32>, b: Finite<f32>, a: Finite<f32>) {
        assert!(self.is_open.get());
        let color = [*r, *g, *b, *a];
        self.ipc_sender.send(WebMetalEncoderCommand::ClearColor(color)).unwrap();
    }
    fn EndEncoding(&self) {
        assert!(self.is_open.get());
        self.is_open.set(false);
        self.ipc_sender.send(WebMetalEncoderCommand::EndEncoding).unwrap();
    }
}
