/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasMsg};
use dom::bindings::codegen::Bindings::WebMetalRenderingContextBinding as binding;
use dom::bindings::js::{LayoutJS, Root};
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::webmetalcommandqueue::WebMetalCommandQueue;
use dom::webmetaldevice::WebMetalDevice;
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};

#[dom_struct]
pub struct WebMetalRenderingContext {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_renderer: IpcSender<CanvasMsg>,
    device: Root<WebMetalDevice>,
    command_queue: Root<WebMetalCommandQueue>,
}

impl WebMetalRenderingContext {
    fn new_internal(_global: &GlobalScope, _canvas: &HTMLCanvasElement, _size: Size2D<i32>)
                    -> Result<WebMetalRenderingContext, String> {
        Err(String::new())
    }

    #[allow(unrooted_must_root)]
    pub fn new(global: &GlobalScope, canvas: &HTMLCanvasElement, size: Size2D<i32>)
               -> Option<Root<WebMetalRenderingContext>> {
        match Self::new_internal(global, canvas, size) {
            Ok(ctx) => Some(reflect_dom_object(box ctx, global, binding::Wrap)),
            Err(msg) => {
                error!("Couldn't create WebMetalRenderingContext: {}", msg);
                //TODO: error event?
                None
            }
        }
    }

    pub fn recreate(&self, size: Size2D<i32>) {
        self.ipc_renderer.send(CanvasMsg::Common(CanvasCommonMsg::Recreate(size))).unwrap();
    }

    pub fn ipc_renderer(&self) -> IpcSender<CanvasMsg> {
        self.ipc_renderer.clone()
    }
}

impl binding::WebMetalRenderingContextMethods for WebMetalRenderingContext {
    fn GetDevice(&self) -> Root<WebMetalDevice> {
        self.device.clone()
    }
    fn GetCommandQueue(&self) -> Root<WebMetalCommandQueue> {
        self.command_queue.clone()
    }
}

pub trait LayoutCanvasWebMetalRenderingContextHelpers {
    #[allow(unsafe_code)]
    unsafe fn get_ipc_renderer(&self) -> IpcSender<CanvasMsg>;
}

impl LayoutCanvasWebMetalRenderingContextHelpers for LayoutJS<WebMetalRenderingContext> {
    #[allow(unsafe_code)]
    unsafe fn get_ipc_renderer(&self) -> IpcSender<CanvasMsg> {
        (*self.unsafe_get()).ipc_renderer.clone()
    }
}
