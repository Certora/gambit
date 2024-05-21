// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract RequireMutation {
    function myRequires(bool cond1, bool cond2, bool cond3) public pure returns (bool) {
	/// RequireMutation(`cond1` |==> `false`) of: `require(cond1);`
	require(false);
	require(cond2);
	require(cond3);
	return true;
    }
}
