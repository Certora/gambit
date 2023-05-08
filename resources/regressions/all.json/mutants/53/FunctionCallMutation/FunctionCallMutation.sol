// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract FunctionCallMutation {
    function myAddition(uint256 x, uint256 y) public pure returns (uint256) {
	/// BinaryOpMutation(`+` |==> `-`) of: `return x + y;`
	return x-y;
    }

    function myOtherAddition(uint256 x, uint256 y) public pure returns (uint256) {
	return myAddition(x, y);
    }
}
