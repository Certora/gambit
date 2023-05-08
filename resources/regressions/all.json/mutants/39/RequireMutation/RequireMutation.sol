// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract RequireMutation {
    function myRequires(bool cond1, bool cond2, bool cond3) public pure returns (bool) {
	require(cond1);
	require(cond2);
	/// RequireMutation(`cond3` |==> `true`) of: `require(cond3);`
	require(true);
	return true;
    }
}
