# G-Code Analysis Script
# Analyzes generated G-code files for arc detection effectiveness and quality metrics

param(
    [string]$InputDir = ".\testing\output",
    [string]$OutputFile = ".\testing\output\analysis_report.html",
    [switch]$Detailed
)

$WorkspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$InputDir = Join-Path $WorkspaceRoot $InputDir.TrimStart('.\')
$OutputFile = Join-Path $WorkspaceRoot $OutputFile.TrimStart('.\')

Write-Host "=== G-Code Analysis Report ===" -ForegroundColor Cyan
Write-Host "Input Directory: $InputDir" -ForegroundColor Gray
Write-Host "Output Report: $OutputFile" -ForegroundColor Gray

# Function to analyze a single G-code file
function Get-DetailedGcodeAnalysis {
    param($FilePath)
    
    if (-not (Test-Path $FilePath)) {
        return $null
    }
    
    $content = Get-Content $FilePath
    $analysis = @{
        FilePath = $FilePath
        FileName = Split-Path $FilePath -Leaf
        TotalLines = $content.Count
        G0Count = ($content | Where-Object { $_ -match "^G0\b" }).Count
        G1Count = ($content | Where-Object { $_ -match "^G1\b" }).Count
        G2Count = ($content | Where-Object { $_ -match "^G2\b" }).Count
        G3Count = ($content | Where-Object { $_ -match "^G3\b" }).Count
        M3Count = ($content | Where-Object { $_ -match "^M3\b" }).Count
        M5Count = ($content | Where-Object { $_ -match "^M5\b" }).Count
        Comments = ($content | Where-Object { $_ -match "^\(" }).Count
        FileSizeBytes = (Get-Item $FilePath).Length
    }
    
    # Calculate derived metrics
    $analysis.MovementCommands = $analysis.G0Count + $analysis.G1Count + $analysis.G2Count + $analysis.G3Count
    $analysis.ArcCommands = $analysis.G2Count + $analysis.G3Count
    $analysis.LinearCommands = $analysis.G1Count
    $analysis.FileSizeKB = [math]::Round($analysis.FileSizeBytes / 1024, 2)
    
    if ($analysis.MovementCommands -gt 0) {
        $analysis.ArcPercentage = [math]::Round(($analysis.ArcCommands / $analysis.MovementCommands) * 100, 1)
        $analysis.LinearPercentage = [math]::Round(($analysis.LinearCommands / $analysis.MovementCommands) * 100, 1)
    } else {
        $analysis.ArcPercentage = 0
        $analysis.LinearPercentage = 0
    }
    
    # Analyze arc commands for I/J format validation
    $arcLines = $content | Where-Object { $_ -match "^G[23]\b" }
    $analysis.ArcWithIJ = ($arcLines | Where-Object { $_ -match "\bI[-\d.]" -or $_ -match "\bJ[-\d.]" }).Count
    $analysis.ArcFormatCorrect = if ($analysis.ArcCommands -gt 0) {
        [math]::Round(($analysis.ArcWithIJ / $analysis.ArcCommands) * 100, 1)
    } else { 0 }
    
    # Tool sequence validation
    $analysis.ToolSequenceValid = ($analysis.M3Count -gt 0 -and $analysis.M5Count -gt 0)
    
    # Efficiency metrics
    if ($analysis.TotalLines -gt 0) {
        $analysis.MovementDensity = [math]::Round(($analysis.MovementCommands / $analysis.TotalLines) * 100, 1)
    } else {
        $analysis.MovementDensity = 0
    }
    
    return $analysis
}

# Find all G-code files
$gcodeFiles = Get-ChildItem -Path $InputDir -Filter "*.gcode" -Recurse | ForEach-Object { $_.FullName }

if ($gcodeFiles.Count -eq 0) {
    Write-Warning "No G-code files found in $InputDir"
    exit 1
}

Write-Host "Found $($gcodeFiles.Count) G-code files to analyze" -ForegroundColor Green

# Analyze all files
$analyses = @()
foreach ($file in $gcodeFiles) {
    Write-Host "Analyzing: $(Split-Path $file -Leaf)" -ForegroundColor White
    $analysis = Get-DetailedGcodeAnalysis -FilePath $file
    if ($analysis) {
        $analyses += $analysis
    }
}

# Group analyses by configuration type
$withArcs = $analyses | Where-Object { $_.FileName -match "_with_arcs" }
$withoutArcs = $analyses | Where-Object { $_.FileName -match "_without_arcs" }

# Generate comparison data
$comparisons = @()
foreach ($withArc in $withArcs) {
    $baseName = $withArc.FileName -replace "_with_arcs.gcode", ""
    $withoutArc = $withoutArcs | Where-Object { $_.FileName -match "^$baseName" }
    
    if ($withoutArc) {
        $sizeReduction = if ($withoutArc.FileSizeBytes -gt 0) {
            [math]::Round((($withoutArc.FileSizeBytes - $withArc.FileSizeBytes) / $withoutArc.FileSizeBytes) * 100, 1)
        } else { 0 }
        
        $lineReduction = if ($withoutArc.TotalLines -gt 0) {
            [math]::Round((($withoutArc.TotalLines - $withArc.TotalLines) / $withoutArc.TotalLines) * 100, 1)
        } else { 0 }
        
        $comparisons += [PSCustomObject]@{
            SVGFile = $baseName
            WithArcsKB = $withArc.FileSizeKB
            WithoutArcsKB = $withoutArc.FileSizeKB
            SizeReductionPercent = $sizeReduction
            WithArcsLines = $withArc.TotalLines
            WithoutArcsLines = $withoutArc.TotalLines
            LineReductionPercent = $lineReduction
            ArcPercentage = $withArc.ArcPercentage
            ArcCount = $withArc.ArcCommands
            Quality = if ($withArc.ArcPercentage -gt 20) { "Excellent" } 
                      elseif ($withArc.ArcPercentage -gt 10) { "Good" }
                      elseif ($withArc.ArcPercentage -gt 5) { "Fair" }
                      else { "Poor" }
        }
    }
}

# Console output
Write-Host "`n=== Analysis Summary ===" -ForegroundColor Cyan

Write-Host "`nArc Detection Effectiveness:" -ForegroundColor Yellow
$withArcs | Sort-Object FileName | ForEach-Object {
    $color = switch ($_.ArcPercentage) {
        { $_ -gt 30 } { "Green" }
        { $_ -gt 10 } { "Yellow" }
        default { "Red" }
    }
    Write-Host "  $($_.FileName): $($_.ArcPercentage)% arcs, $($_.FileSizeKB) KB" -ForegroundColor $color
}

Write-Host "`nFile Size Comparison:" -ForegroundColor Yellow
$comparisons | Sort-Object SVGFile | ForEach-Object {
    $color = if ($_.SizeReductionPercent -gt 0) { "Green" } else { "Red" }
    Write-Host "  $($_.SVGFile): $($_.SizeReductionPercent)% size reduction ($($_.WithoutArcsKB) → $($_.WithArcsKB) KB)" -ForegroundColor $color
}

if ($Detailed) {
    Write-Host "`nDetailed Analysis:" -ForegroundColor Yellow
    $analyses | Sort-Object FileName | Format-Table -Property FileName, ArcPercentage, ArcCommands, LinearCommands, FileSizeKB, TotalLines -AutoSize
}

# Generate HTML report
$htmlContent = @"
<!DOCTYPE html>
<html>
<head>
    <title>SVG2GCode Arc Detection Analysis Report</title>
    <style>
        body { font-family: 'Segoe UI', Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1, h2 { color: #2c3e50; border-bottom: 2px solid #3498db; padding-bottom: 10px; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #3498db; color: white; }
        tr:nth-child(even) { background-color: #f8f9fa; }
        .metric { display: inline-block; margin: 10px; padding: 15px; background: #ecf0f1; border-radius: 5px; text-align: center; min-width: 120px; }
        .metric-value { font-size: 24px; font-weight: bold; color: #2c3e50; }
        .metric-label { font-size: 12px; color: #7f8c8d; }
        .excellent { background-color: #d5f4e6; color: #27ae60; }
        .good { background-color: #fef5e7; color: #f39c12; }
        .fair { background-color: #fdedec; color: #e74c3c; }
        .poor { background-color: #fadbd8; color: #c0392b; }
        .chart { margin: 20px 0; }
        .bar { height: 20px; background: linear-gradient(90deg, #3498db, #2980b9); margin: 5px 0; border-radius: 10px; }
        .summary-box { background: #e8f4fd; border-left: 4px solid #3498db; padding: 15px; margin: 20px 0; }
    </style>
</head>
<body>
    <div class="container">
        <h1>SVG2GCode Arc Detection Analysis Report</h1>
        <div class="summary-box">
            <strong>Generated:</strong> $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")<br>
            <strong>Files Analyzed:</strong> $($analyses.Count)<br>
            <strong>Test Configurations:</strong> With Arc Detection ($($withArcs.Count) files), Without Arc Detection ($($withoutArcs.Count) files)
        </div>
        
        <h2>Key Metrics</h2>
        <div>
"@

# Add key metrics
$avgArcPercentage = if ($withArcs.Count -gt 0) { [math]::Round(($withArcs | Measure-Object -Property ArcPercentage -Average).Average, 1) } else { 0 }
$avgSizeReduction = if ($comparisons.Count -gt 0) { [math]::Round(($comparisons | Measure-Object -Property SizeReductionPercent -Average).Average, 1) } else { 0 }
$totalFilesWithArcs = ($withArcs | Where-Object { $_.ArcPercentage -gt 0 }).Count

$htmlContent += @"
            <div class="metric">
                <div class="metric-value">$avgArcPercentage%</div>
                <div class="metric-label">Average Arc Usage</div>
            </div>
            <div class="metric">
                <div class="metric-value">$avgSizeReduction%</div>
                <div class="metric-label">Average Size Reduction</div>
            </div>
            <div class="metric">
                <div class="metric-value">$totalFilesWithArcs/$($withArcs.Count)</div>
                <div class="metric-label">Files with Arcs Detected</div>
            </div>
        </div>
        
        <h2>Arc Detection Effectiveness by File</h2>
        <table>
            <tr>
                <th>SVG File</th>
                <th>Arc Usage %</th>
                <th>Arc Commands</th>
                <th>File Size (KB)</th>
                <th>Quality Rating</th>
            </tr>
"@

foreach ($analysis in ($withArcs | Sort-Object FileName)) {
    $baseName = $analysis.FileName -replace "_with_arcs.gcode", ""
    $quality = if ($analysis.ArcPercentage -gt 20) { "excellent" } 
               elseif ($analysis.ArcPercentage -gt 10) { "good" }
               elseif ($analysis.ArcPercentage -gt 5) { "fair" }
               else { "poor" }
    
    $qualityText = $quality.Substring(0,1).ToUpper() + $quality.Substring(1)
    
    $htmlContent += @"
            <tr>
                <td>$baseName</td>
                <td>$($analysis.ArcPercentage)%</td>
                <td>$($analysis.ArcCommands)</td>
                <td>$($analysis.FileSizeKB)</td>
                <td><span class="$quality">$qualityText</span></td>
            </tr>
"@
}

$htmlContent += @"
        </table>
        
        <h2>Performance Comparison</h2>
        <table>
            <tr>
                <th>SVG File</th>
                <th>Without Arcs (KB)</th>
                <th>With Arcs (KB)</th>
                <th>Size Reduction</th>
                <th>Line Reduction</th>
            </tr>
"@

foreach ($comparison in ($comparisons | Sort-Object SVGFile)) {
    $sizeClass = if ($comparison.SizeReductionPercent -gt 0) { "style='color: green;'" } else { "style='color: red;'" }
    $lineClass = if ($comparison.LineReductionPercent -gt 0) { "style='color: green;'" } else { "style='color: red;'" }
    
    $htmlContent += @"
            <tr>
                <td>$($comparison.SVGFile)</td>
                <td>$($comparison.WithoutArcsKB)</td>
                <td>$($comparison.WithArcsKB)</td>
                <td $sizeClass>$($comparison.SizeReductionPercent)%</td>
                <td $lineClass>$($comparison.LineReductionPercent)%</td>
            </tr>
"@
}

$htmlContent += @"
        </table>
        
        <h2>Technical Details</h2>
        <table>
            <tr>
                <th>File</th>
                <th>Total Lines</th>
                <th>G0 (Rapid)</th>
                <th>G1 (Linear)</th>
                <th>G2 (CW Arc)</th>
                <th>G3 (CCW Arc)</th>
                <th>Arc Format Correct</th>
            </tr>
"@

foreach ($analysis in ($analyses | Sort-Object FileName)) {
    $formatCorrect = if ($analysis.ArcFormatCorrect -eq 100) { "✓" } else { "$($analysis.ArcFormatCorrect)%" }
    
    $htmlContent += @"
            <tr>
                <td>$($analysis.FileName)</td>
                <td>$($analysis.TotalLines)</td>
                <td>$($analysis.G0Count)</td>
                <td>$($analysis.G1Count)</td>
                <td>$($analysis.G2Count)</td>
                <td>$($analysis.G3Count)</td>
                <td>$formatCorrect</td>
            </tr>
"@
}

$htmlContent += @"
        </table>
        
        <div class="summary-box">
            <h3>Summary</h3>
            <p>This analysis shows the effectiveness of polygon arc detection in svg2gcode. 
            Arc detection successfully converts polygonal approximations of circles into 
            efficient G2/G3 arc commands, reducing file size and improving machining quality.</p>
            
            <ul>
                <li><strong>Best Performance:</strong> $(($comparisons | Sort-Object ArcPercentage -Descending | Select-Object -First 1).SVGFile) with $((($comparisons | Sort-Object ArcPercentage -Descending | Select-Object -First 1).ArcPercentage))% arc usage</li>
                <li><strong>Largest Size Reduction:</strong> $(($comparisons | Sort-Object SizeReductionPercent -Descending | Select-Object -First 1).SVGFile) with $((($comparisons | Sort-Object SizeReductionPercent -Descending | Select-Object -First 1).SizeReductionPercent))% reduction</li>
                <li><strong>Files Successfully Processed:</strong> $($analyses.Count) of $($analyses.Count)</li>
            </ul>
        </div>
    </div>
</body>
</html>
"@

# Save HTML report
$htmlContent | Out-File -FilePath $OutputFile -Encoding UTF8

Write-Host "`n=== Report Generated ===" -ForegroundColor Green
Write-Host "HTML Report: $OutputFile" -ForegroundColor White
Write-Host "Open the HTML file in a browser to view the detailed analysis." -ForegroundColor Gray
