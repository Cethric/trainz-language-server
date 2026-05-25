package lsp

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import org.jetbrains.annotations.NotNull

class TrainzLanguageClient(@NotNull project: Project) : LanguageClientImpl(project) {
    
}