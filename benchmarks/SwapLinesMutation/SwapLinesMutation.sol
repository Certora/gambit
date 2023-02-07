// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract SwapLinesMutation {
    function addFifteen(uint256 x) public pure returns (uint256) {
	x += 1;
	x += 2;
	x += 3;
	x += 4;
	x += 5;
	return x;
    }
}
