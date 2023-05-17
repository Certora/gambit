// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract RequireMutation {
    function myRequires(bool cond1, bool cond2, bool cond3) public pure returns (bool) {
	require(cond1);
	/// RequireMutation(`cond2` |==> `false`) of: `require(cond2);`
	require(false);
	require(cond3);
	return true;
    }
}
