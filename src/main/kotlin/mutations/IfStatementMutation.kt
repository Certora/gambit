package mutations

import data.SolNode
import kotlin.random.Random


object IfStatementMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "IfStatement"
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not mutation point for IfStatementMutation" }
        val condition = node.condition()!!
        return if (rand.nextBoolean()) {
            condition.replaceInSource(source, listOf("true", "false").random(rand))
        } else {
            condition.replaceInSource(source, "!(" + condition.getText(source) + ")")
        }
    }
}
