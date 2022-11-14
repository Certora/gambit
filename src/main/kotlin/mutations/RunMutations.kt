package mutations

import data.SolNode
import kotlin.random.Random
import java.nio.file.Files
import java.nio.file.Path
import java.util.*
import kotlin.io.path.writeText
import log.Logger
import java.lang.Integer.min

val allMutations = listOf(
    IntegerMutation,
    ArithmeticBinaryOpMutation,
    PredicateBinaryOpMutation,
    LogicalBinaryOperatorMutation,
    DeleteExpressionMutation,
    IfStatementMutation,
    FunctionCallMutation,
    AssignmentMutation,
    SwapArgumentsFunctionMutation,
    UncheckedBlockMutation,
    RequireMutation,
    SwapLinesMutation,
    SwapArgumentsOperatorMutation,
    UnaryOperatorMutation
)

private const val ATTEMPTS_FACTOR = 50
private val CLASS_MAP = allMutations.associateBy { it.javaClass.simpleName.lowercase() }
private val CLASS_MAP_WITHOUT_MUTATION = CLASS_MAP.mapKeys { it.key.removeSuffix("mutation") }

// Mutants can be specified with or without "mutation" at the end of the name
fun mutationTypesFromStrings(strings: List<String>): List<Mutation> {
    return strings.map {
        when (it) {
            in CLASS_MAP -> {
                CLASS_MAP.getValue(it.lowercase())
            }
            in CLASS_MAP_WITHOUT_MUTATION -> {
                CLASS_MAP_WITHOUT_MUTATION.getValue(it.lowercase())
            }
            else -> {
                throw IllegalArgumentException("Unknown mutant name: $it")
            }
        }
    }
}
                    

class RunMutations(val contractFile: Path,
                   val solNode: SolNode,
                   val rand: Random,
                   val numMutants: Int,
                   val outputFolder: Path,
                   val functionsToMutate: List<String>) {
    
    fun isAssertCall(node: SolNode): Boolean {
        return node.name() == "assert"
    }

    fun getIndent(line: String): String {
        val result = StringBuilder()
        for (c in line) {
            if (c.isWhitespace()) {
                result.append(c)
            } else {
                break
            }
        }
        return result.toString()
    }

    // Add a comment which tells the user what the line was before the mutation
    fun processMutant(sourceArray: ByteArray, mutant: String, mutation: Mutation): String {
        val source = String(sourceArray, Charsets.UTF_8)
        val scan1 = Scanner(source)
        val scan2 = Scanner(mutant)
        val result = StringBuilder()
        while (scan1.hasNextLine() && scan2.hasNextLine()) {
            val line1 = scan1.nextLine() + "\n"
            val line2 = scan2.nextLine() + "\n"
            if (line1 != line2) {
                val indent = getIndent(line1)
                val line1WithoutIndent = line1.removePrefix(indent)
                result.append(indent + "/// " + mutation.javaClass.simpleName + " of: " + line1WithoutIndent)
                result.append(line2)
                break
            }
            result.append(line2)
        }
        while (scan2.hasNextLine()) {
            result.append(scan2.nextLine() + "\n")
        }

        return result.toString()
    }

    // Attempts to give you exactly numMutant number of valid mutants, and sampled uniformly from
    // all the possible kinds of mutants in allMutations
    fun getMutations(mutationTypes: List<Mutation>, validMutant: (Path) -> Boolean): List<Path> {
        val source = Files.readAllBytes(contractFile)
        val mutationPoints: Map<Mutation, List<SolNode>> = solNode.traverse(
            { node ->
                mutationTypes.filter { mutation ->
                    mutation.isMutationPoint(node)
                }.map { it to node }.takeIf { it.isNotEmpty() }
            },
            { node -> isAssertCall(node) },
            { node ->
                functionsToMutate.isEmpty() ||
                        (node.nodeType() == "FunctionDefinition" && node.name() in functionsToMutate)
            })
            .flatten().groupBy({ it.first }, { it.second }).takeIf { it.isNotEmpty() }
            ?: return run {
                Logger.always("No possible mutations found", respectQuiet = true)
                listOf()
            }

        // a queue of mutations to perform
        val mutationPointsTodo = ArrayDeque<Mutation>()
        val pointList: List<Mutation> = mutationPoints.toList().map { it.first }
        check(pointList.isNotEmpty()) { "Should have mutants at this point" }
        var remaining = numMutants
        while (remaining > 0) {
            mutationPointsTodo.addAll(pointList.take(min(remaining, pointList.size)))
            remaining -= pointList.size
        }

        var attempts = 0
        val mutants: MutableList<Path> = mutableListOf()
        val seen: MutableSet<String> = mutableSetOf()
        seen.add(String(source, Charsets.UTF_8))
        // Try to generate mutants, give up after ATTEMPTS_FACTOR*numMutants attempts
        while (mutationPointsTodo.isNotEmpty() && attempts < ATTEMPTS_FACTOR * numMutants) {
            val mutation = mutationPointsTodo.removeFirst()
            val points = mutationPoints[mutation] ?: throw IllegalStateException("Found unexpected mutation")
            val point = points.random(rand)

            // Perform the chosen mutation
            val mutantOriginal = mutation.mutateRandomly(point, source, rand)
            val mutant = processMutant(source, mutantOriginal, mutation)
            val file = outputFolder.resolve("${contractFile.fileName}_${attempts}.sol")
            file.writeText(mutant)
            if (seen.contains(mutantOriginal)) {
                // skip this mutant
            } else if (validMutant(file)) {
                // Add this mutant because it compiled
                mutants.add(file)
            } else {
                // Try this type of mutation again at a later point
                mutationPointsTodo.add(mutation)
            }

            seen.add(mutantOriginal)

            attempts++
        }

        return mutants.toList()
    }
}