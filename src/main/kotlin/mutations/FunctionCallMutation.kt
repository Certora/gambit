package mutations

import data.SolNode
import kotlin.random.Random


object FunctionCallMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "FunctionCall" && node.arguments()!!.isNotEmpty()
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for FunctionCallMutation" }
        return node.replaceInSource(source, node.arguments()!!.random(rand).getText(source))
    }
}
