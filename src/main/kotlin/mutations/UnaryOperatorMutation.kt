package mutations

import data.SolNode
import kotlin.random.Random

object UnaryOperatorMutation: Mutation() {
    private val prefixOps = setOf("++", "--", "~")
    private val suffixOps = setOf("++", "--")
    private fun isPrefix(source: String, operator: String): Boolean {
        // if its a prefix, it will start with the first, possibly only index of the operator
        return (source.get(0) == operator.get(0))
    }
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "UnaryOperation"
    }

    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not mutation point for UnaryOperatorMutation" }
        val (start, end) = node.getBounds()
        return if (isPrefix(node.getText(source), node.operator()!!)) {
            // change the prefix
            val tmp = node.replacePart(source, prefixOps.random(rand), start, start + node.operator()!!.length)
            tmp
        } else {

            val tmp = node.replacePart(source, suffixOps.random(rand), 
                            end - node.operator()!!.length, end)
            tmp
        }
    }
}