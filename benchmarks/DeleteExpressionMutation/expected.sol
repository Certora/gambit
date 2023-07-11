// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract DeleteExpressionMutation {

    function myIdentity(uint256 x) public pure returns (uint256) {
	uint256 result = 0;
	/// DeleteExpressionMutation of: for (uint256 i = 0; i < x; i++) {
	for (uint256 i = 0; i < x; assert(true)) {
	    result ++;
	}
	return result;
    }
}
