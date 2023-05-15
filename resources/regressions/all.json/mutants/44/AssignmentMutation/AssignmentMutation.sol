// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract AssignmentMutation {
    uint256 public x;
    uint256 public y;
    uint256 public z;
    bool public a;
    bool public b;

    constructor() {
	x = 42; // original: 42
	/// AssignmentMutation(`13` |==> `1`) of: `y = 13; // original: 13`
	y = 1; // original: 13
	z = 3110; // original: 3110
	a = true; // original: true
	b = false; // original: false
    }
}
