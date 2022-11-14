package mutations

import data.SolNode
import kotlin.random.Random
import kotlin.random.nextULong

object IntegerMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        val attr = node.getTypeDescriptions() ?: return false
        val typeString = attr.typeString()
        return typeString != null && typeString.startsWith("int_const ") && typeString.split(" ")[1].toULongOrNull() != null
    }

    private fun getULong(node: SolNode): ULong? {
        return node.getTypeDescriptions()?.typeString()?.split(" ")?.get(1)?.toULong()
    }

    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for IntegerMutation" }
        val oldVal = getULong(node)!!
        val newVal = if (rand.nextBoolean()) {
            oldVal + 1UL
        } else if (rand.nextBoolean()) {
            oldVal - 1UL
        } else {
            rand.nextULong(0UL, ULong.MAX_VALUE)
        }
        return node.replaceInSource(source, newVal.toString())
    }
}
