/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuImageState;
typedef unsigned long WebGpuBufferState;

dictionary WebGpuBufferBarrier {
	required WebGpuBufferState stateSrc;
	required WebGpuBufferState stateDst;
	required WebGpuBuffer target;
	//TODO: offset/size
};

dictionary WebGpuImageBarrier {
	required WebGpuImageState stateSrc;
	required WebGpuImageState stateDst;
	required WebGpuImage target;
	//TODO: subresource range
};

interface WebGpuCommandBuffer {
	WebGpuSubmit finish();
	void pipelineBarrier(
		sequence<WebGpuBufferBarrier> buffers,
		sequence<WebGpuImageBarrier> images
	);
};
