# SVG2GCode Comprehensive Test Runner
# This script tests polygon arc detection functionality across multiple SVG files and settings

param(
    [string]$TestCategory = "all",  # all, functional, performance, regression
    [switch]$Verbose,
    [switch]$CleanOutput
)

# Configuration
$WorkspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$TestingDir = Join-Path $WorkspaceRoot "testing"
$SvgSamplesDir = Join-Path $TestingDir "svg-samples"
$SettingsDir = Join-Path $TestingDir "settings"
$OutputDir = Join-Path $TestingDir "output"
$ScriptsDir = Join-Path $TestingDir "scripts"

# Ensure we're in the right directory
Set-Location $WorkspaceRoot

Write-Host "=== SVG2GCode Polygon Arc Detection Test Suite ===" -ForegroundColor Cyan
Write-Host "Workspace: $WorkspaceRoot" -ForegroundColor Gray
Write-Host "Test Category: $TestCategory" -ForegroundColor Gray

# Clean output directory if requested
if ($CleanOutput) {
    Write-Host "Cleaning output directory..." -ForegroundColor Yellow
    if (Test-Path $OutputDir) {
        Remove-Item "$OutputDir\*" -Recurse -Force
    }
}

# Create output directories
@("with_arcs", "without_arcs", "performance", "comparison") | ForEach-Object {
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

# Test configuration
$testConfigs = @(
    @{
        Name = "Arc Detection Enabled"
        Settings = "polygon_arc_enabled.json"
        OutputSuffix = "_with_arcs"
        OutputDir = "with_arcs"
    },
    @{
        Name = "Arc Detection Disabled"
        Settings = "polygon_arc_disabled.json"
        OutputSuffix = "_without_arcs"
        OutputDir = "without_arcs"
    }
)

# SVG test files with categories
$svgFiles = @(
    @{ Path = "polygonal\svg-1.svg"; Category = "polygonal"; ExpectedArcs = "High" },
    @{ Path = "polygonal\svg-2.svg"; Category = "polygonal"; ExpectedArcs = "Medium" },
    @{ Path = "polygonal\test_circles.svg"; Category = "polygonal"; ExpectedArcs = "High" },
    @{ Path = "mixed\test_snakes.svg"; Category = "mixed"; ExpectedArcs = "Low" }
)

# Function to run G-code generation
function Invoke-GcodeGeneration {
    param($SvgPath, $SettingsPath, $OutputPath)
    
    $fullSvgPath = Join-Path $SvgSamplesDir $SvgPath
    $fullSettingsPath = Join-Path $SettingsDir $SettingsPath
    
    if (-not (Test-Path $fullSvgPath)) {
        Write-Warning "SVG file not found: $fullSvgPath"
        return $false
    }
    
    if (-not (Test-Path $fullSettingsPath)) {
        Write-Warning "Settings file not found: $fullSettingsPath"
        return $false
    }
    
    if ($Verbose) {
        Write-Host "  Running: svg2gcode '$fullSvgPath' --settings '$fullSettingsPath' --out '$OutputPath'" -ForegroundColor Gray
    }
    
    $result = & "$WorkspaceRoot\target\release\svg2gcode.exe" $fullSvgPath --settings $fullSettingsPath --out $OutputPath 2>&1
    
    if ($LASTEXITCODE -eq 0 -and (Test-Path $OutputPath)) {
        return $true
    } else {
        Write-Warning "Failed to generate G-code for $SvgPath"
        if ($Verbose -and $result) {
            Write-Host "Error output: $result" -ForegroundColor Red
        }
        return $false
    }
}

# Function to analyze G-code file
function Get-GcodeStats {
    param($FilePath)
    
    if (-not (Test-Path $FilePath)) {
        return @{ Error = "File not found" }
    }
    
    $content = Get-Content $FilePath
    $totalLines = $content.Count
    $g0Count = ($content | Select-String "^G0" | Measure-Object).Count
    $g1Count = ($content | Select-String "^G1" | Measure-Object).Count
    $g2Count = ($content | Select-String "^G2" | Measure-Object).Count
    $g3Count = ($content | Select-String "^G3" | Measure-Object).Count
    
    $movementCommands = $g0Count + $g1Count + $g2Count + $g3Count
    $arcCommands = $g2Count + $g3Count
    $arcPercentage = if ($movementCommands -gt 0) { [math]::Round(($arcCommands / $movementCommands) * 100, 1) } else { 0 }
    
    return @{
        TotalLines = $totalLines
        G0Count = $g0Count
        G1Count = $g1Count
        G2Count = $g2Count
        G3Count = $g3Count
        ArcCount = $arcCommands
        MovementCount = $movementCommands
        ArcPercentage = $arcPercentage
        FileSizeKB = [math]::Round((Get-Item $FilePath).Length / 1024, 2)
    }
}

# Main test execution
$testResults = @()

Write-Host "`n=== Running Functional Tests ===" -ForegroundColor Green

foreach ($config in $testConfigs) {
    Write-Host "`nTesting with: $($config.Name)" -ForegroundColor Yellow
    
    foreach ($svg in $svgFiles) {
        $svgName = [System.IO.Path]::GetFileNameWithoutExtension($svg.Path)
        $outputFile = Join-Path $OutputDir $config.OutputDir "$svgName$($config.OutputSuffix).gcode"
        
        Write-Host "  Processing: $($svg.Path)" -ForegroundColor White
        
        $success = Invoke-GcodeGeneration -SvgPath $svg.Path -SettingsPath $config.Settings -OutputPath $outputFile
        
        if ($success) {
            $stats = Get-GcodeStats -FilePath $outputFile
            
            $testResults += [PSCustomObject]@{
                SvgFile = $svg.Path
                Configuration = $config.Name
                Success = $true
                ArcPercentage = $stats.ArcPercentage
                ArcCount = $stats.ArcCount
                FileSizeKB = $stats.FileSizeKB
                TotalLines = $stats.TotalLines
                Category = $svg.Category
                ExpectedArcs = $svg.ExpectedArcs
            }
            
            if ($Verbose) {
                Write-Host "    ✓ Generated $($stats.TotalLines) lines, $($stats.ArcPercentage)% arcs" -ForegroundColor Green
            }
        } else {
            $testResults += [PSCustomObject]@{
                SvgFile = $svg.Path
                Configuration = $config.Name
                Success = $false
                ArcPercentage = 0
                ArcCount = 0
                FileSizeKB = 0
                TotalLines = 0
                Category = $svg.Category
                ExpectedArcs = $svg.ExpectedArcs
            }
        }
    }
}

# Performance comparison
if ($TestCategory -eq "all" -or $TestCategory -eq "performance") {
    Write-Host "`n=== Performance Comparison ===" -ForegroundColor Green
    
    $performanceResults = @()
    
    foreach ($svg in $svgFiles) {
        $svgName = [System.IO.Path]::GetFileNameWithoutExtension($svg.Path)
        
        # Test with arcs enabled
        $withArcsFile = Join-Path $OutputDir "with_arcs" "$svgName`_with_arcs.gcode"
        $withoutArcsFile = Join-Path $OutputDir "without_arcs" "$svgName`_without_arcs.gcode"
        
        if ((Test-Path $withArcsFile) -and (Test-Path $withoutArcsFile)) {
            $withArcsStats = Get-GcodeStats -FilePath $withArcsFile
            $withoutArcsStats = Get-GcodeStats -FilePath $withoutArcsFile
            
            $sizeReduction = if ($withoutArcsStats.FileSizeKB -gt 0) {
                [math]::Round((($withoutArcsStats.FileSizeKB - $withArcsStats.FileSizeKB) / $withoutArcsStats.FileSizeKB) * 100, 1)
            } else { 0 }
            
            $performanceResults += [PSCustomObject]@{
                SvgFile = $svg.Path
                WithArcsKB = $withArcsStats.FileSizeKB
                WithoutArcsKB = $withoutArcsStats.FileSizeKB
                SizeReductionPercent = $sizeReduction
                ArcPercentage = $withArcsStats.ArcPercentage
            }
        }
    }
    
    Write-Host "Performance Results:" -ForegroundColor Cyan
    $performanceResults | Format-Table -AutoSize
}

# Generate summary report
Write-Host "`n=== Test Summary ===" -ForegroundColor Cyan

$successfulTests = ($testResults | Where-Object { $_.Success }).Count
$totalTests = $testResults.Count

Write-Host "Tests Passed: $successfulTests / $totalTests" -ForegroundColor $(if ($successfulTests -eq $totalTests) { "Green" } else { "Red" })

# Arc detection effectiveness
$arcEnabledResults = $testResults | Where-Object { $_.Configuration -eq "Arc Detection Enabled" -and $_.Success }
$arcDisabledResults = $testResults | Where-Object { $_.Configuration -eq "Arc Detection Disabled" -and $_.Success }

Write-Host "`nArc Detection Effectiveness:" -ForegroundColor Yellow
$arcEnabledResults | ForEach-Object {
    $color = switch ($_.ArcPercentage) {
        { $_ -gt 30 } { "Green" }
        { $_ -gt 10 } { "Yellow" }
        default { "Red" }
    }
    Write-Host "  $($_.SvgFile): $($_.ArcPercentage)% arcs detected" -ForegroundColor $color
}

# Detailed results table
if ($Verbose) {
    Write-Host "`nDetailed Results:" -ForegroundColor Cyan
    $testResults | Format-Table -AutoSize
}

# Validation checks
Write-Host "`n=== Validation Results ===" -ForegroundColor Cyan

$validationErrors = @()

# Check that arc-enabled configs actually produce arcs for polygonal SVGs
$polygonalWithArcs = $testResults | Where-Object { 
    $_.Category -eq "polygonal" -and 
    $_.Configuration -eq "Arc Detection Enabled" -and 
    $_.Success 
}

foreach ($result in $polygonalWithArcs) {
    if ($result.ArcPercentage -lt 5) {
        $validationErrors += "Low arc detection for polygonal SVG: $($result.SvgFile) (only $($result.ArcPercentage)%)"
    }
}

# Check that arc-disabled configs don't produce many arcs
$disabledWithArcs = $testResults | Where-Object { 
    $_.Configuration -eq "Arc Detection Disabled" -and 
    $_.Success -and 
    $_.ArcPercentage -gt 5 
}

foreach ($result in $disabledWithArcs) {
    $validationErrors += "Unexpected arcs when disabled: $($result.SvgFile) ($($result.ArcPercentage)%)"
}

if ($validationErrors.Count -eq 0) {
    Write-Host "✓ All validation checks passed" -ForegroundColor Green
} else {
    Write-Host "✗ Validation errors found:" -ForegroundColor Red
    $validationErrors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
}

# Save results to file
$resultsFile = Join-Path $OutputDir "test_results.json"
$testResults | ConvertTo-Json -Depth 3 | Out-File $resultsFile
Write-Host "`nTest results saved to: $resultsFile" -ForegroundColor Gray

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Generated files available in: $OutputDir" -ForegroundColor Gray

# Return appropriate exit code
if ($validationErrors.Count -eq 0 -and $successfulTests -eq $totalTests) {
    exit 0
} else {
    exit 1
}
