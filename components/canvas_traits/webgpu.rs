/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
use webgpu_component::{gpu, QueueType};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct WebGpuContextId(pub usize);

impl ::heapsize::HeapSizeOf for WebGpuContextId {
    fn heap_size_of_children(&self) -> usize { 0 }
}

pub type WebGpuSender<T> = ipc_channel::ipc::IpcSender<T>;
pub type WebGpuReceiver<T> = ipc_channel::ipc::IpcReceiver<T>;

pub fn webgpu_channel<T: Serialize + for<'de> Deserialize<'de>>(
) -> Result<(WebGpuSender<T>, WebGpuReceiver<T>), io::Error>
{
    ipc_channel::ipc::channel()
}

#[derive(Clone, Deserialize, Serialize)]
pub struct QueueInfo {
    pub ty: QueueType,
    pub count: u8,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AdapterInfo {
    pub info: gpu::AdapterInfo,
    pub queue_families: Vec<QueueInfo>,
}

/// Contains the WebGpuCommand sender and information about a WebGpuContext
#[derive(Clone, Deserialize, Serialize)]
pub struct WebGpuInit {
    /// Sender instance to send commands to the specific WebGpuContext.
    pub sender: WebGpuMsgSender,
    /// Vector of available adapters.
    pub adapters: Vec<AdapterInfo>,
}

/// WebGpu Message API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuMsg {
    /// Creates a new WebGPU context instance.
    CreateContext(WebGpuSender<Result<WebGpuInit, String>>),
    /// Frees all resources and closes the thread.
    Exit,
}

pub type WebGpuChan = WebGpuSender<WebGpuMsg>;

#[derive(Clone, Deserialize, Serialize)]
pub struct WebGpuPipeline(pub WebGpuChan);

impl WebGpuPipeline {
    pub fn channel(&self) -> WebGpuChan {
        self.0.clone()
    }
}

/// Helper struct to send WebGpuCommands to a specific WebGpuContext.
#[derive(Clone, Deserialize, HeapSizeOf, Serialize)]
pub struct WebGpuMsgSender {
    ctx_id: WebGpuContextId,
    #[ignore_heap_size_of = "channels are hard"]
    sender: WebGpuChan,
}

impl WebGpuMsgSender {
    pub fn new(ctx_id: WebGpuContextId, sender: WebGpuChan) -> Self {
        WebGpuMsgSender {
            ctx_id,
            sender,
        }
    }
}
