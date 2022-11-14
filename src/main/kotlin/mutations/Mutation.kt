package mutations

import data.SolNode
import kotlin.random.Random

abstract class Mutation {
    abstract fun isMutationPoint(node: SolNode): Boolean
    
    abstract fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String
}
