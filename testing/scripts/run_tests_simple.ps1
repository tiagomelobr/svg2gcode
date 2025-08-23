# SVG2GCode Comprehensive Test Runner
# This script tests polygon arc detection functionality across multiple SVG files and settings

param(
    [string]$TestCategory = "all",
    [switch]$Verbose,
    [switch]$CleanOutput
)

# Configuration
$WorkspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$TestingDir = Join-Path $WorkspaceRoot "testing"
$SvgSamplesDir = Join-Path $TestingDir "svg-samples"
$SettingsDir = Join-Path $TestingDir "settings"
$OutputDir = Join-Path $TestingDir "output"

# Ensure we're in the right directory
Set-Location $WorkspaceRoot

Write-Host "=== SVG2GCode Polygon Arc Detection Test Suite ===" -ForegroundColor Cyan
Write-Host "Workspace: $WorkspaceRoot" -ForegroundColor Gray

# Clean output directory if requested
if ($CleanOutput) {
    Write-Host "Cleaning output directory..." -ForegroundColor Yellow
    if (Test-Path $OutputDir) {
        Remove-Item "$OutputDir\*" -Recurse -Force
    }
}

# Create output directories
@("with_arcs", "without_arcs") | ForEach-Object {
    $dir = Join-Path $OutputDir $_
    if (-not (Test-Path $dir)) {
        New-Item -Path $dir -ItemType Directory -Force | Out-Null
    }
}

# Build the project first
Write-Host "Building svg2gcode..." -ForegroundColor Yellow
$buildResult = cargo build --release --bin svg2gcode
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed. Aborting tests."
    exit 1
}

# Test SVG files
$svgFiles = @(
    "polygonal\svg-1.svg",
    "polygonal\svg-2.svg", 
    "polygonal\test_circles.svg",
    "mixed\test_snakes.svg"
)

# Function to analyze G-code file
function Get-GcodeStats {
    param($FilePath)
    
    if (-not (Test-Path $FilePath)) {
        return @{ Error = "File not found" }
    }
    
    $content = Get-Content $FilePath
    $totalLines = $content.Count
    $g1Count = ($content | Select-String "^G1" | Measure-Object).Count
    $g2Count = ($content | Select-String "^G2" | Measure-Object).Count  
    $g3Count = ($content | Select-String "^G3" | Measure-Object).Count
    
    $movementCommands = $g1Count + $g2Count + $g3Count
    $arcCommands = $g2Count + $g3Count
    $arcPercentage = if ($movementCommands -gt 0) { 
        [math]::Round(($arcCommands / $movementCommands) * 100, 1) 
    } else { 0 }
    
    return @{
        TotalLines = $totalLines
        G1Count = $g1Count
        G2Count = $g2Count
        G3Count = $g3Count
        ArcCount = $arcCommands
        ArcPercentage = $arcPercentage
        FileSizeKB = [math]::Round((Get-Item $FilePath).Length / 1024, 2)
    }
}

# Test with arc detection enabled
Write-Host "`n=== Testing with Arc Detection Enabled ===" -ForegroundColor Green

$withArcsResults = @()
foreach ($svgFile in $svgFiles) {
    $svgName = [System.IO.Path]::GetFileNameWithoutExtension((Split-Path $svgFile -Leaf))
    $fullSvgPath = Join-Path $SvgSamplesDir $svgFile
    $outputPath = Join-Path (Join-Path $OutputDir "with_arcs") "$svgName`_with_arcs.gcode"
    $settingsPath = Join-Path $SettingsDir "polygon_arc_enabled.json"
    
    Write-Host "Processing: $svgFile" -ForegroundColor White
    
    if (Test-Path $fullSvgPath) {
        $result = & "$WorkspaceRoot\target\release\svg2gcode.exe" $fullSvgPath --settings $settingsPath --out $outputPath 2>&1
        
        if ($LASTEXITCODE -eq 0 -and (Test-Path $outputPath)) {
            $stats = Get-GcodeStats -FilePath $outputPath
            $withArcsResults += [PSCustomObject]@{
                SVGFile = $svgFile
                Success = $true
                ArcPercentage = $stats.ArcPercentage
                ArcCount = $stats.ArcCount
                FileSizeKB = $stats.FileSizeKB
                TotalLines = $stats.TotalLines
            }
            Write-Host "  Success: $($stats.ArcPercentage)% arcs, $($stats.FileSizeKB) KB" -ForegroundColor Green
        } else {
            $withArcsResults += [PSCustomObject]@{
                SVGFile = $svgFile
                Success = $false
                ArcPercentage = 0
                ArcCount = 0
                FileSizeKB = 0
                TotalLines = 0
            }
            Write-Host "  Failed to generate G-code" -ForegroundColor Red
        }
    } else {
        Write-Host "  SVG file not found: $fullSvgPath" -ForegroundColor Red
    }
}

# Test with arc detection disabled  
Write-Host "`n=== Testing with Arc Detection Disabled ===" -ForegroundColor Green

$withoutArcsResults = @()
foreach ($svgFile in $svgFiles) {
    $svgName = [System.IO.Path]::GetFileNameWithoutExtension((Split-Path $svgFile -Leaf))
    $fullSvgPath = Join-Path $SvgSamplesDir $svgFile
    $outputPath = Join-Path (Join-Path $OutputDir "without_arcs") "$svgName`_without_arcs.gcode"
    $settingsPath = Join-Path $SettingsDir "polygon_arc_disabled.json"
    
    Write-Host "Processing: $svgFile" -ForegroundColor White
    
    if (Test-Path $fullSvgPath) {
        $result = & "$WorkspaceRoot\target\release\svg2gcode.exe" $fullSvgPath --settings $settingsPath --out $outputPath 2>&1
        
        if ($LASTEXITCODE -eq 0 -and (Test-Path $outputPath)) {
            $stats = Get-GcodeStats -FilePath $outputPath
            $withoutArcsResults += [PSCustomObject]@{
                SVGFile = $svgFile
                Success = $true
                ArcPercentage = $stats.ArcPercentage
                ArcCount = $stats.ArcCount
                FileSizeKB = $stats.FileSizeKB
                TotalLines = $stats.TotalLines
            }
            Write-Host "  Success: $($stats.ArcPercentage)% arcs, $($stats.FileSizeKB) KB" -ForegroundColor Green
        } else {
            $withoutArcsResults += [PSCustomObject]@{
                SVGFile = $svgFile
                Success = $false
                ArcPercentage = 0
                ArcCount = 0
                FileSizeKB = 0
                TotalLines = 0
            }
            Write-Host "  Failed to generate G-code" -ForegroundColor Red
        }
    }
}

# Generate comparison report
Write-Host "`n=== Test Summary ===" -ForegroundColor Cyan

$successfulWithArcs = ($withArcsResults | Where-Object { $_.Success }).Count
$successfulWithoutArcs = ($withoutArcsResults | Where-Object { $_.Success }).Count
$totalTests = $svgFiles.Count * 2

Write-Host "Tests Passed: $($successfulWithArcs + $successfulWithoutArcs) / $totalTests" -ForegroundColor $(if (($successfulWithArcs + $successfulWithoutArcs) -eq $totalTests) { "Green" } else { "Red" })

Write-Host "`nArc Detection Effectiveness:" -ForegroundColor Yellow
$withArcsResults | Where-Object { $_.Success } | ForEach-Object {
    $color = if ($_.ArcPercentage -gt 20) { "Green" } elseif ($_.ArcPercentage -gt 5) { "Yellow" } else { "Red" }
    Write-Host "  $($_.SVGFile): $($_.ArcPercentage)% arcs detected" -ForegroundColor $color
}

Write-Host "`nFile Size Comparison:" -ForegroundColor Yellow
foreach ($i in 0..($svgFiles.Count - 1)) {
    $withArcs = $withArcsResults[$i]
    $withoutArcs = $withoutArcsResults[$i]
    
    if ($withArcs.Success -and $withoutArcs.Success) {
        $reduction = if ($withoutArcs.FileSizeKB -gt 0) {
            [math]::Round((($withoutArcs.FileSizeKB - $withArcs.FileSizeKB) / $withoutArcs.FileSizeKB) * 100, 1)
        } else { 0 }
        
        $color = if ($reduction -gt 0) { "Green" } else { "Red" }
        Write-Host "  $($svgFiles[$i]): $reduction% size reduction ($($withoutArcs.FileSizeKB) -> $($withArcs.FileSizeKB) KB)" -ForegroundColor $color
    }
}

# Validation checks
Write-Host "`n=== Validation Results ===" -ForegroundColor Cyan

$validationErrors = @()

# Check arc detection works for polygonal files
$polygonalFiles = $withArcsResults | Where-Object { $_.SVGFile -like "*polygonal*" -and $_.Success }
foreach ($result in $polygonalFiles) {
    if ($result.ArcPercentage -lt 5) {
        $validationErrors += "Low arc detection for polygonal SVG: $($result.SVGFile)"
    }
}

# Check disabled configs don't produce many arcs
$disabledWithArcs = $withoutArcsResults | Where-Object { $_.Success -and $_.ArcPercentage -gt 5 }
foreach ($result in $disabledWithArcs) {
    $validationErrors += "Unexpected arcs when disabled: $($result.SVGFile)"
}

if ($validationErrors.Count -eq 0) {
    Write-Host "All validation checks passed!" -ForegroundColor Green
} else {
    Write-Host "Validation errors found:" -ForegroundColor Red
    $validationErrors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Generated files available in: $OutputDir" -ForegroundColor Gray

# Return appropriate exit code
if ($validationErrors.Count -eq 0 -and ($successfulWithArcs + $successfulWithoutArcs) -eq $totalTests) {
    exit 0
} else {
    exit 1
}
