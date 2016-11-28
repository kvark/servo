/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasMsg, WebMetalCommand};
use dom::bindings::codegen::Bindings::WebMetalRenderingContextBinding as binding;
use dom::bindings::js::{JS, LayoutJS, Root};
use dom::bindings::inheritance::Castable;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::node::{Node, NodeDamage};
use dom::webmetalcommandbuffer::WebMetalCommandBuffer;
use dom::webmetaldevice::WebMetalDevice;
use dom::webmetaltargetview::WebMetalTargetView;
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};
use script_traits::ScriptMsg as ConstellationMsg;
use std::cell::Cell;
use webmetal::WebMetalCapabilities;

#[dom_struct]
pub struct WebMetalRenderingContext {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_renderer: IpcSender<CanvasMsg>,
    #[ignore_heap_size_of = "Defined in webmetal"]
    capabilities: WebMetalCapabilities,
    canvas: JS<HTMLCanvasElement>,
    device: JS<WebMetalDevice>,
    current_target_index: Cell<usize>,
    swap_targets: Vec<JS<WebMetalTargetView>>,
}

impl WebMetalRenderingContext {
    fn new_internal(global: &GlobalScope, canvas: &HTMLCanvasElement, size: Size2D<i32>)
                    -> Result<WebMetalRenderingContext, String> {
        let (sender, receiver) = ipc::channel().unwrap();
        let num_frames = 3;
        global.constellation_chan()
              .send(ConstellationMsg::CreateWebMetalPaintThread(size, num_frames, sender))
              .unwrap();
        let response = receiver.recv().unwrap();
        response.map(|(ipc_renderer, targets, caps)| WebMetalRenderingContext {
            reflector: Reflector::new(),
            ipc_renderer: ipc_renderer.clone(),
            capabilities: caps,
            canvas: JS::from_ref(canvas),
            device: JS::from_ref(&*WebMetalDevice::new(global, ipc_renderer)),
            current_target_index: Cell::new(0),
            swap_targets: targets.into_iter().map(|view|
                JS::from_ref(&*WebMetalTargetView::new(global, view))
                ).collect(),
        })
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
        Root::from_ref(&self.device)
    }

    fn MakeCommandBuffer(&self) -> Root<WebMetalCommandBuffer> {
        let (sender, receiver) = ipc::channel().unwrap();
        let msg = WebMetalCommand::MakeCommandBuffer(sender);
        self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
        let inner = receiver.recv().unwrap().unwrap();
        WebMetalCommandBuffer::new(&self.global(), self.ipc_renderer.clone(), inner)
    }

    fn NextFrameTarget(&self) -> Root<WebMetalTargetView> {
        let mut index = self.current_target_index.get() + 1;
        if index >= self.swap_targets.len() {
            index = 0;
        }
        self.current_target_index.set(index);
        Root::from_ref(&self.swap_targets[index])
    }

    fn EndFrame(&self) {
        let msg = WebMetalCommand::Present(self.current_target_index.get() as u32);
        self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
        //TODO: wait for a fence
        self.canvas.upcast::<Node>().dirty(NodeDamage::OtherNodeDamage);
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
