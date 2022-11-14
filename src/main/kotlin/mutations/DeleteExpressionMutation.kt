package mutations

import data.SolNode
import kotlin.random.Random


object DeleteExpressionMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "ExpressionStatement"
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not mutation point for DeleteExpressionMutation" }
        return node.commentOut(source)
    }
}
