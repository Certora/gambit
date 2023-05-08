// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract DeleteExpressionMutation {

    function myIdentity(uint256 x) public pure returns (uint256) {
	uint256 result = 0;
	/// SwapArgumentsOperatorMutation(`i < x` |==> `x < i`) of: `for (uint256 i = 0; i < x; i++) {`
	for (uint256 i = 0; x < i; i++) {
	    result ++;
	}
	return result;
    }
}
