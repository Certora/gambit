package mutations

import data.SolNode
import kotlin.random.Random


object RequireMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "FunctionCall" &&
               node.expression()?.name() == "require" &&
               node.arguments()!!.isNotEmpty()
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for RequireMutation" }
        val arg = node.arguments()!![0]
        return arg.replaceInSource(source, "!(" + arg.getText(source) + ")")
    }
}
