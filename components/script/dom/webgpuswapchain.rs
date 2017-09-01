/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{SwapchainInfo, WebGpuChan, WebGpuMsg};
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::WebGpuSwapchainBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::{WebGpuFormat, WebGpuFramebufferSize, WebGpuSemaphore};
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpuheap::WebGpuHeap;
use dom::webgpuimage::WebGpuImage;
use dom_struct::dom_struct;
use euclid::Size2D;


pub struct IdRotation {
    total: binding::WebGpuSwapchainImageId,
    acquire: binding::WebGpuSwapchainImageId,
    present: Option<binding::WebGpuSwapchainImageId>,
}

impl IdRotation {
    fn new(total: binding::WebGpuSwapchainImageId) -> Self {
        IdRotation {
            total,
            acquire: 0,
            present: None,
        }
    }

    fn acquire(&mut self) -> Option<binding::WebGpuSwapchainImageId> {
        let id = self.acquire;
        if Some(id) != self.present {
            if self.present.is_none() {
                self.present = Some(id);
            }
            self.acquire += 1;
            if self.acquire >= self.total {
                self.acquire = 0;
            }
            Some(id)
        } else {
            None
        }
    }

    fn present(&mut self) -> Option<binding::WebGpuSwapchainImageId> {
        self.present
            .take()
            .map(|id| {
                let mut next = id + 1;
                if next >= self.total {
                    next = 0;
                }
                if next != self.acquire {
                    self.present = Some(next);
                }
                id
            })
    }
}


#[dom_struct]
pub struct WebGpuSwapchain {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    format: WebGpuFormat,
    size: Size2D<u32>,
    heap: Root<WebGpuHeap>,
    images: Vec<Root<WebGpuImage>>,
    #[ignore_heap_size_of = "Nothing to see here"]
    id_rotation: DOMRefCell<IdRotation>,
}

impl WebGpuSwapchain {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        format: WebGpuFormat,
        size: Size2D<u32>,
        swapchain: SwapchainInfo,
    ) -> Root<Self>
    {
        let count = swapchain.images.len();
        let obj = box WebGpuSwapchain {
            reflector_: Reflector::new(),
            sender,
            format,
            size,
            heap: WebGpuHeap::new(global, swapchain.heap_id),
            images: swapchain.images
                .into_iter()
                .map(|id| WebGpuImage::new(global, id))
                .collect(),
            id_rotation: DOMRefCell::new(IdRotation::new(count as _)),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuSwapchainMethods for WebGpuSwapchain {
    fn Format(&self) -> WebGpuFormat {
        self.format.clone()
    }

    fn GetSize(&self) -> WebGpuFramebufferSize {
        WebGpuFramebufferSize {
            width: self.size.width,
            height: self.size.height,
            layers: self.images.len() as _,
        }
    }

    fn AcquireNextImage(&self,
        _semaphore: WebGpuSemaphore,
    ) -> binding::WebGpuSwapchainImageId
    {
        //TODO: semaphore
        self.id_rotation.borrow_mut().acquire().unwrap()
    }

    fn GetImages(&self) -> Vec<Root<WebGpuImage>> {
        self.images.clone()
    }

    fn Present(&self) {
        let id = self.id_rotation.borrow_mut().present().unwrap();
        let image_key = self.images[id as usize].get_id();
        let msg = WebGpuMsg::Present(image_key);
        self.sender.send(msg).unwrap();
    }
}
