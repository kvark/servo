/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::WebMetalEncoderCommand;
use dom::bindings::codegen::Bindings::WebMetalRenderEncoderBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalrenderpipelinestate::WebMetalRenderPipelineState;
use ipc_channel::ipc::IpcSender;
use std::cell::Cell;
use webmetal;

#[dom_struct]
pub struct WebMetalRenderEncoder {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in webmetal"]
    pass: webmetal::RenderPass,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_sender: IpcSender<WebMetalEncoderCommand>,
    is_open: Cell<bool>,
}

impl WebMetalRenderEncoder {
    pub fn new(global: &GlobalScope,
               pass: webmetal::RenderPass,
               ipc_sender: IpcSender<WebMetalEncoderCommand>)
               -> Root<WebMetalRenderEncoder> {
        let object = box WebMetalRenderEncoder {
            reflector: Reflector::new(),
            pass: pass,
            ipc_sender: ipc_sender,
            is_open: Cell::new(true),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalRenderEncoderMethods for WebMetalRenderEncoder {
    fn SetRenderPipelineState(&self, pipeline: &WebMetalRenderPipelineState) {
        assert!(self.is_open.get());
        let handle = pipeline.get(&self.pass);
        self.ipc_sender.send(WebMetalEncoderCommand::SetPipeline(handle)).unwrap();
    }

    fn DrawPrimitives(&self, start: u32, count: u32, instances: u32) {
        assert!(self.is_open.get());
        self.ipc_sender.send(WebMetalEncoderCommand::Draw(start, count, instances)).unwrap();
    }

    fn EndEncoding(&self) {
        assert!(self.is_open.get());
        self.is_open.set(false);
        self.ipc_sender.send(WebMetalEncoderCommand::EndEncoding).unwrap();
    }
}
