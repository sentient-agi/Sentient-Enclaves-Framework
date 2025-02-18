import logging
import sys
from logging.handlers import RotatingFileHandler
from pathlib import Path

def setup_logger(log_dir: str = "logs") -> logging.Logger:
    """
    Set up and configure project-wide logger
    
    Args:
        log_dir: Directory where log files will be stored
    Returns:
        Logger instance
    """
    # Create logs directory if it doesn't exist
    Path(log_dir).mkdir(parents=True, exist_ok=True)
    
    # Create logger
    logger = logging.getLogger('twitter_agent')
    logger.setLevel(logging.INFO)
    
    # Prevent adding handlers multiple times
    if logger.handlers:
        return logger
    
    # Create formatters
    file_formatter = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(filename)s:%(lineno)d - %(message)s'
    )
    console_formatter = logging.Formatter(
        '%(asctime)s - %(levelname)s - %(message)s'
    )
    
    # Create and configure file handler (rotating log files)
    file_handler = RotatingFileHandler(
        filename=f"{log_dir}/twitter_agent.log",
        maxBytes=10*1024*1024,  # 10MB
        backupCount=5
    )
    file_handler.setLevel(logging.DEBUG)
    file_handler.setFormatter(file_formatter)
    
    # Create and configure console handler
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(logging.INFO)
    console_handler.setFormatter(console_formatter)
    
    # Add handlers to logger
    logger.addHandler(file_handler)
    logger.addHandler(console_handler)
    
    return logger

# Create a global logger instance
logger = setup_logger() 