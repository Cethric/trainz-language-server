package lang

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/**
 * File type for Trainz GameScript files (*.gs).
 */
object GameScriptFileType : LanguageFileType(GameScriptLanguage) {
    private fun readResolve(): Any = GameScriptFileType

    override fun getName(): String = "GameScript"

    override fun getDescription(): String = "Trainz game script"

    override fun getDefaultExtension(): String = "gs"

    override fun getIcon(): Icon = IconLoader.getIcon("/icons/gs.svg", GameScriptFileType::class.java)
}
