import uniffi.goose_uniffi.*

fun main() {
  val msgs = listOf(
    Message(role = "user", text = "Hello, how are you?"),
    Message(role = "assistant", text = "I'm fine, thanks! How can I help?")
  )

  try {
    val tooltip = generateTooltip(msgs)
    println("Tooltip: $tooltip")
  } catch (e: ProviderException) {
    println("Error generating tooltip: ${e}")
  }
}
