/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

interface WebGpuCommandQueue {    
	WebGpuCommandPool createCommandPool(unsigned long maxCommandBuffers);
	void submit(
		sequence<WebGpuSubmit> commandBuffers,
		sequence<WebGpuSemaphore> waitSemaphores,
		sequence<WebGpuSemaphore> signalSemaphores,
		WebGpuFence? fence);
};
