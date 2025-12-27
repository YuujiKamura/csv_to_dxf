# CSV to DXF Road Section Converter

![Application Preview](screenshots/app_preview.png)

This tool automatically generates DXF files of road cross-sections from CSV data. It allows for easy conversion of survey data or road design data into CAD drawings.

## Key Features

- **CSV Import**: Load road survey data from CSV files
- **Section Extraction**: Select and extract target sections from multiple sections in a file
- **DXF Generation**: Output road cross-sections in CAD-compatible DXF format
- **Batch Processing**: Support for batch processing of multiple sections
- **GUI/CLI Support**: Both PyQt6 GUI and command-line interfaces available
- **Windows EXE**: Standalone application that runs without Python environment

## Processing Options

| Option | Description |
|--------|-------------|
| Auto-convert distances | Convert incremental distances to cumulative distances |
| Normalize station names | Standardize station names to sequential format (No.0, No.1, ...) |
| Swap left/right widths | Swap left and right width data for specific sections |
| Width assignment | Select left or right width |

## Installation

### Python Environment Setup

```bash
# Clone the repository
git clone https://github.com/YuujiKamura/csv_to_dxf.git
cd csv_to_dxf

# Install required libraries
pip install pandas ezdxf PyQt6 requests
```

### Windows Direct Execution

No Python installation required:

1. Clone or download the GitHub repository
2. Run `dist/csv_to_dxf/csv_to_dxf.exe`

## Usage

### GUI Application

1. Start the application:

```bash
# Using Python
python main.py

# Or on Windows, using the executable
dist/csv_to_dxf/csv_to_dxf.exe
```

2. Click the "Browse..." button to select a CSV file
3. Select the desired section from the dropdown menu
4. Click the "Preview" button to preview the data
5. Click the "Convert to DXF" button to generate the DXF file
   - The file save dialog will automatically populate with the selected section name as the default filename

### Command-line Operation

```bash
# Convert road section data from clipboard to DXF
python src/table_to_dxf.py
```

## Input Data Format

### CSV File Format

The input CSV file requires the following fields:

| Column | Description | Example |
|--------|-------------|---------|
| Section name | Name to identify the section | Section1, Section2 |
| Station name | Name of the measurement point | No.0, No.1+10 |
| Distance | Distance between stations (m) | 10, 20.5 |
| Width | Road width (m) | 3.50, 4.20 |

### Clipboard Input Format

4-column data format:

| Station Name | Distance (m) | Left Width (m) | Right Width (m) |
|--------------|--------------|----------------|-----------------|
| No.0         | 0            | 3.50           | 3.35            |
| No.1         | 10           | 3.40           | 3.35            |

**Notes:**
- For one-sided width data, enter 0 for the width on the other side
- Decimal values are processed accurately

## Project Structure

```
csv_to_dxf/
├── main.py                    # GUI application entry point
├── app_config.json            # Application settings
│
├── gui/                       # GUI application
│   ├── app_controller.py      # Application controller
│   ├── core/                  # Core functionality modules
│   │   ├── csv_processor.py   # CSV processing
│   │   ├── dxf_generator.py   # DXF generation
│   │   └── config_manager.py  # Configuration management
│   └── ui/                    # UI components
│       └── main_window.py     # Main window
│
├── src/                       # Core processing modules
│   ├── processing.py          # CSV extraction and conversion
│   ├── exporter.py            # File export
│   ├── dxf_draw_tenkaiz.py    # DXF drawing engine
│   ├── station_name_utils.py  # Station name utilities
│   └── table_to_dxf.py        # Clipboard to DXF conversion
│
├── modules/                   # Specialized modules
│   └── board_collector/       # Board photo collection feature
│
├── tests/                     # Test suite
│   ├── e2e/                   # End-to-end tests
│   ├── integration/           # Integration tests
│   └── unit/                  # Unit tests
│
├── data/                      # Sample data
├── dist/csv_to_dxf/           # Built Windows application
└── generate_dxf_kouyama.py    # Google Sheets integration script
```

## Special Features

### Google Sheets Integration

Use `generate_dxf_kouyama.py` to fetch data directly from Google Sheets and generate DXF:

```bash
python generate_dxf_kouyama.py
```

### Board Photo Collection

The `modules/board_collector/` module allows collection of construction photo board data.

### Screenshot Feature

Save preview screens to the `screenshots/` directory from the menu "Save Screenshot".

## Testing

### Automated Tests

```bash
# Run all tests
python -m pytest tests/

# Run individual tests
python -m pytest tests/e2e/test_csv_to_dxf.py
python -m pytest tests/integration/test_app_controller.py
```

### Manual Testing

```bash
# Run test script
python tests/test_save_dxf.py
```

## Output File

The generated DXF file can be opened with the following software:

- AutoCAD
- Jw_cad
- LibreCAD
- Other CAD software that supports DXF

## System Requirements

- **Python**: 3.6 or higher
- **OS**: Windows, Linux, macOS

### Dependencies

| Library | Purpose |
|---------|---------|
| pandas | Data processing |
| ezdxf | DXF generation |
| PyQt6 | GUI (GPL License) |
| requests | HTTP communication (for Google Sheets) |

## Troubleshooting

### "Could not load data from clipboard"

- Verify that data has been copied to clipboard correctly
- Check that the data format has 4 columns
- Make sure to select the cell range accurately in the spreadsheet before copying

### "DXF file is not created"

- Check the console output for detailed error messages
- Review the input data format
- Make sure existing DXF files are not open (which may prevent overwriting)

## Changelog

### 2025-04-30: Windows Executable (EXE) Added

- Added Windows executable (EXE) file
- Standalone application that runs without Python environment

### 2025-04-29: DXF Saving Function Update

- Fixed parameter mismatch in `CSVProcessor.save_dxf()` method
- Fixed DataFrame boolean evaluation error in `get_section()` method
- Enhanced error and success logging

## License

This software is released under the GNU General Public License (GPL) version 3.

**Note**: This application uses PyQt6, which is provided under the GPL license. As a result, this entire application (both source code and binary) is subject to the terms of the GPL. GPL is a copyleft license, which means that any distributed derivative works must also be distributed under the same license terms (GPL) with source code availability.

## Copyright

© 2025 Yuuji Softworks
