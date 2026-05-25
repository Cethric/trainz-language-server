package lang

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/**
 * File type for Trainz config files (config.txt).
 */
object TrainzConfigFileType : LanguageFileType(TrainzConfigLanguage) {
    private fun readResolve(): Any = TrainzConfigFileType

    override fun getName(): String = "TrainzConfig"

    override fun getDescription(): String = "Trainz config"

    override fun getDefaultExtension(): String = "txt"

    override fun getIcon(): Icon = IconLoader.getIcon("/icons/acs.svg", TrainzConfigFileType::class.java)
}
