// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.13;

contract Helper {
    function setVars(uint _num) public payable {}
}

contract EDC {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;

    /// ElimDelegateCall(`delegatecall` |==> `call`) of: `function setVars(address _contract) public payable {`
    function setVars(address _contract) public payable {
        (bool success, ) = _contract.call(
            abi.encodeWithSignature("setVars(uint256)", 1)
        );
        require(success, "Delegatecall failed");
    }
}
