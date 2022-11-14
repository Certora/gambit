package mutations

import data.SolNode
import kotlin.random.Random

abstract class BinaryOpMutation : Mutation() {
    protected abstract val ops : Set<String>
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "BinaryOperation"
    }
    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for OperatorMutation" }
        val (_, endl) = node.leftExpression()!!.getBounds()
        val (startr, _) = node.rightExpression()!!.getBounds()
        return node.replacePart(source, " " + ops.random(rand) + " ", endl, startr)
        
    }
}