/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuCommandPoolFlags;

interface WebGpuCommandQueue {
	const WebGpuCommandPoolFlags	COMMAND_POOL_TRANSIENT			= 0x1;
	const WebGpuCommandPoolFlags	COMMAND_POOL_RESET_INDIVIDUAL	= 0x2;

	WebGpuCommandPool createCommandPool(WebGpuCommandPoolFlags flags);
	void submit(
		sequence<WebGpuCommandBuffer> commandBuffers,
		sequence<WebGpuSemaphore> waitSemaphores,
		sequence<WebGpuSemaphore> signalSemaphores,
		WebGpuFence? fence);
};
