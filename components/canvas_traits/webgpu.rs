/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct WebGpuContextId(pub usize);

impl ::heapsize::HeapSizeOf for WebGpuContextId {
    fn heap_size_of_children(&self) -> usize { 0 }
}

pub type WebGpuSender<T> = ipc_channel::ipc::IpcSender<T>;
pub type WebGpuReceiver<T> = ipc_channel::ipc::IpcReceiver<T>;

pub fn webgl_channel<T: Serialize + for<'de> Deserialize<'de>>()
        -> Result<(WebGpuSender<T>, WebGpuReceiver<T>), io::Error> {
    ipc_channel::ipc::channel()
}

/// WebGpu Message API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuMsg {
    /// Frees all resources and closes the thread.
    Exit,
}

/// Helper struct to send WebGpuCommands to a specific WebGpuContext.
#[derive(Clone, Deserialize, HeapSizeOf, Serialize)]
pub struct WebGpuMsgSender {
    ctx_id: WebGpuContextId,
    #[ignore_heap_size_of = "channels are hard"]
    sender: WebGpuSender<WebGpuMsg>,
}
