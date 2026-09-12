param([Parameter(Mandatory = $true)][string]$Llama)
$ErrorActionPreference = 'Stop'
$Patch = (Resolve-Path (Join-Path $PSScriptRoot '../patches/llama-inferenza.patch')).Path

# La patch e' versionata nell'app: il riferimento del submodule non cambia.
$GiaApplicata = & {
    # Su Windows PowerShell 5 stderr di un comando nativo puo' essere un errore
    # terminante; qui un check negativo e' previsto sui sorgenti ancora puliti.
    $ErrorActionPreference = 'Continue'
    git -C $Llama apply --reverse --check $Patch 2>$null
    $LASTEXITCODE -eq 0
}
if ($GiaApplicata) { return }
git -C $Llama apply --check $Patch
if ($LASTEXITCODE -ne 0) { throw 'Patch inferenza incompatibile con questi sorgenti llama.cpp' }
git -C $Llama apply $Patch
if ($LASTEXITCODE -ne 0) { throw 'Applicazione patch inferenza fallita' }
