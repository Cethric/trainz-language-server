package lang

import com.intellij.lang.Language

/**
 * Represents the GameScript language used by Trainz (.gs files).
 */
object GameScriptLanguage : Language("GameScript", "text/x-gamescript") {
    private fun readResolve(): Any = GameScriptLanguage

    override fun getDisplayName(): String = "GameScript"
}
