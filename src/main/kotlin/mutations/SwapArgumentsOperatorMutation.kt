package mutations

import data.SolNode
import kotlin.random.Random

object SwapArgumentsOperatorMutation : Mutation() {
    private val nonCommutativeOps = setOf("-", "/", "%", "**", ">",
                                         "<", ">=", "<=", "<<", ">>")

    override fun isMutationPoint(node: SolNode) : Boolean {
        return (node.nodeType() == "BinaryOperation" && nonCommutativeOps.contains(node.operator()))
    }
    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for SwapArgumentsOperatorMutation" }
        val leftChild = node.leftExpression()!!
        val rightChild = node.rightExpression()!!
        return node.replaceMultiple(source,
            listOf(Pair(leftChild, rightChild.getText(source)),
            Pair(rightChild, leftChild.getText(source))))
    }

}