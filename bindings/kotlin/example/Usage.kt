import kotlinx.coroutines.runBlocking
import uniffi.goose_uniffi.*

fun main() = runBlocking {
    val msgs = listOf(
        Message(
            role    = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("Add 2 + 3")))
        ),
        Message(
            role    = Role.ASSISTANT,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("I’m fine, thanks! How can I help?")))
        ), 
        Message(
            role    = Role.USER,
            created = System.currentTimeMillis() / 1000,
            content = listOf(MessageContent.Text(TextContent("Why is the sky blue? Tell me in less than 20 words.")))
        ),
    )

    printMessages(msgs)
    println("---\n")

    val calculatorTool = createToolConfig(
        name = "calculator",
        inputSchema = """
            {
                "type": "object",
                "required": ["operation", "numbers"],
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"],
                        "description": "The arithmetic operation to perform"
                    },
                    "numbers": {
                        "type": "array",
                        "items": { "type": "number" },
                        "description": "List of numbers to operate on in order"
                    }
                }
            }
        """.trimIndent(),
    )

    val extensions = listOf(
        ExtensionConfig(
            "calculator_extension",
            listOf(calculatorTool)
        )
    )

    asyncPrint(msgs, extensions)
}
