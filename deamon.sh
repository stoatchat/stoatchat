#!/bin/bash





# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
PID_DIR="/tmp/stoatchat"
LOG_DIR="/tmp/stoatchat/logs"

# Services configuration
declare -A SERVICES=(
    ["delta"]="revolt-delta"
    ["bonfire"]="revolt-bonfire"
    ["autumn"]="revolt-autumn"
    ["january"]="revolt-january"
    ["gifbox"]="revolt-gifbox"
    ["crond"]="revolt-crond"
    ["pushd"]="revolt-pushd"
    ["voice-ingress"]="revolt-voice-ingress"
)

# Create directories if they don't exist
mkdir -p "$PID_DIR" "$LOG_DIR"

# Function to print colored ASCII art
print_ascii_art() {
    echo -e "${CYAN}"
    echo ' ____  _              _   '
echo '/ ___|| |_ ___   __ _| |_ '
echo '\___ \| __/ _ \ / _` | __|'
echo ' ___) | || (_) | (_| | |_ '
echo '|____/ \__\___/ \__,_|\__|'
echo '                          '
    echo -e "${NC}"
}

# Function to print service status
print_status() {
    local service=$1
    local status=$2
    local message=${3:-""}
    
    case $status in
        "starting")
            echo -e "${BLUE}[INFO]${NC} Starting $service..."
            ;;
        "started")
            echo -e "${GREEN}[OK]${NC} $service started successfully"
            ;;
        "stopped")
            echo -e "${GREEN}[OK]${NC} $service stopped"
            ;;
        "stopping")
            echo -e "${YELLOW}[INFO]${NC} Stopping $service..."
            ;;
        "error")
            echo -e "${RED}[ERROR]${NC} $service: $message"
            ;;
        "running")
            echo -e "${GREEN}[RUNNING]${NC} $service is running (PID: $message)"
            ;;
        "not_running")
            echo -e "${RED}[STOPPED]${NC} $service is not running"
            ;;
    esac
}

# Function to get PID file path
get_pid_file() {
    local service=$1
    echo "$PID_DIR/$service.pid"
}

# Function to get log file path
get_log_file() {
    local service=$1
    echo "$LOG_DIR/$service.log"
}

# Function to check if service is running
is_service_running() {
    local service=$1
    local pid_file=$(get_pid_file $service)
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "$pid"
            return 0
        else
            rm -f "$pid_file"
        fi
    fi
    return 1
}

# Function to start a service
start_service() {
    local service=$1
    local binary=${SERVICES[$service]}
    
    if [ -z "$binary" ]; then
        print_status "$service" "error" "Unknown service"
        return 1
    fi
    
    # Check if already running
    local pid=$(is_service_running $service)
    if [ -n "$pid" ]; then
        print_status "$service" "running" "$pid"
        return 0
    fi
    
    print_status "$service" "starting"
    
    # Start the service with proper error handling
    cargo run --bin "$binary" > "$(get_log_file $service)" 2>&1 &
    local pid=$!
    
    # Save PID immediately
    echo $pid > "$(get_pid_file $service)"
    
    # Wait longer and check if it started successfully
    echo -e "${BLUE}[INFO]${NC} Waiting for $service to initialize..."
    sleep 5
    
    if kill -0 "$pid" 2>/dev/null; then
        print_status "$service" "started"
        echo -e "${CYAN}[INFO]${NC} Use '$0 logs $service' to monitor logs"
        return 0
    else
        # Check if there was an error in the log
        if [ -f "$(get_log_file $service)" ]; then
            echo -e "${YELLOW}[INFO]${NC} Checking startup logs for $service..."
            local error=$(tail -10 "$(get_log_file $service)" | grep -i -E "(error|failed|panic|connection)" | head -1)
            if [ -n "$error" ]; then
                print_status "$service" "error" "$error"
            else
                print_status "$service" "error" "Process exited unexpectedly"
            fi
            echo -e "${CYAN}[INFO]${NC} Check full logs with: '$0 logs $service'"
        else
            print_status "$service" "error" "Failed to start"
        fi
        rm -f "$(get_pid_file $service)"
        return 1
    fi
}

# Function to stop a service
stop_service() {
    local service=$1
    local pid=$(is_service_running $service)
    
    if [ $? -ne 0 ]; then
        print_status "$service" "not_running"
        return 1
    fi
    
    print_status "$service" "stopping"
    
    # Send SIGTERM first
    kill "$pid" 2>/dev/null
    
    # Wait up to 10 seconds for graceful shutdown
    local count=0
    while [ $count -lt 10 ] && kill -0 "$pid" 2>/dev/null; do
        sleep 1
        count=$((count + 1))
    done
    
    # If still running, force kill
    if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid" 2>/dev/null
        sleep 1
    fi
    
    # Remove PID file
    rm -f "$(get_pid_file $service)"
    
    if ! kill -0 "$pid" 2>/dev/null; then
        print_status "$service" "stopped"
        return 0
    else
        print_status "$service" "error" "Failed to stop"
        return 1
    fi
}

# Function to restart a service
restart_service() {
    local service=$1
    stop_service "$service"
    sleep 1
    start_service "$service"
}

# Function to show service status
show_status() {
    echo -e "${PURPLE}=== StoatChat Service Status ===${NC}"
    echo
    
    for service in "${!SERVICES[@]}"; do
        local pid=$(is_service_running $service)
        if [ -n "$pid" ]; then
            print_status "$service" "running" "$pid"
        else
            print_status "$service" "not_running"
        fi
    done
}

# Function to start all services
start_all() {
    echo -e "${PURPLE}=== Starting StoatChat Services ===${NC}"
    print_ascii_art
    echo
    
    check_dependencies
    
    echo -e "${CYAN}Starting all services...${NC}"
    echo
    
    local failed_services=()
    
    for service in "${!SERVICES[@]}"; do
        if ! start_service "$service"; then
            failed_services+=("$service")
        fi
        echo
    done
    
    if [ ${#failed_services[@]} -eq 0 ]; then
        echo -e "${GREEN}=== All services started successfully ===${NC}"
        echo -e "${CYAN}Use '$0 status' to check status${NC}"
        echo -e "${CYAN}Use '$0 stop' to stop all services${NC}"
    else
        echo -e "${RED}=== Some services failed to start ===${NC}"
        echo -e "${RED}Failed: ${failed_services[*]}${NC}"
    fi
}

# Function to stop all services
stop_all() {
    echo -e "${PURPLE}=== Stopping StoatChat Services ===${NC}"
    echo
    
    for service in "${!SERVICES[@]}"; do
        stop_service "$service"
    done
    
    echo
    echo -e "${GREEN}=== All services stopped ===${NC}"
}

# Function to show logs
show_logs() {
    local service=$1
    local log_file=$(get_log_file $service)
    
    if [ ! -f "$log_file" ]; then
        echo -e "${RED}[ERROR]${NC} No log file found for $service"
        return 1
    fi
    
    echo -e "${CYAN}=== Logs for $service ===${NC}"
    tail -f "$log_file"
}

# Function to check dependencies
check_dependencies() {
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}[ERROR]${NC} Cargo not found. Please install Rust/Cargo first."
        exit 1
    fi
    
    echo -e "${GREEN}[OK]${NC} Dependencies checked"
    
    # Check for external service dependencies
    echo -e "${BLUE}[INFO]${NC} Checking external service dependencies..."
    
    local missing_services=()
    
    # Check MongoDB
    if ! nc -z 127.0.0.1 27017 2>/dev/null; then
        missing_services+=("MongoDB (127.0.0.1:27017)")
    fi
    
    # Check Redis
    if ! nc -z 127.0.0.1 6379 2>/dev/null; then
        missing_services+=("Redis (127.0.0.1:6379)")
    fi
    
    # Check RabbitMQ
    if ! nc -z 127.0.0.1 5672 2>/dev/null; then
        missing_services+=("RabbitMQ (127.0.0.1:5672)")
    fi
    
    if [ ${#missing_services[@]} -gt 0 ]; then
        echo -e "${YELLOW}[WARNING]${NC} Some external services are not running:"
        for service in "${missing_services[@]}"; do
            echo -e "${YELLOW}[WARNING]${NC} - $service"
        done
        echo -e "${YELLOW}[WARNING]${NC} Services may fail to start without these dependencies."
        echo -e "${CYAN}[INFO]${NC} You can start them with: docker compose up -d"
        echo
    else
        echo -e "${GREEN}[OK]${NC} All external dependencies are running"
    fi
    echo
}

# Function to show usage
show_usage() {
    echo -e "${PURPLE}=== StoatChat Daemon Script ===${NC}"
    echo
    echo "Usage: $0 {start|stop|restart|status|logs} [service_name]"
    echo
    echo "Commands:"
    echo "  start [service]   Start all services or specific service"
    echo "  stop [service]    Stop all services or specific service"
    echo "  restart [service] Restart all services or specific service"
    echo "  status            Show status of all services"
    echo "  logs <service>    Show logs for specific service (follow mode)"
    echo
    echo "Available services:"
    for service in "${!SERVICES[@]}"; do
        echo "  - $service (${SERVICES[$service]})"
    done
    echo
    echo "Examples:"
    echo "  $0 start           # Start all services"
    echo "  $0 start delta     # Start only delta service"
    echo "  $0 stop            # Stop all services"
    echo "  $0 status          # Show all service status"
    echo "  $0 logs delta      # Follow delta service logs"
}

# Main execution logic
main() {
    local command=${1:-"help"}
    local service_name=${2:-""}
    
    case $command in
        "start")
            if [ -n "$service_name" ]; then
                check_dependencies
                start_service "$service_name"
            else
                start_all
            fi
            ;;
        "stop")
            if [ -n "$service_name" ]; then
                stop_service "$service_name"
            else
                stop_all
            fi
            ;;
        "restart")
            if [ -n "$service_name" ]; then
                check_dependencies
                restart_service "$service_name"
            else
                stop_all
                sleep 2
                start_all
            fi
            ;;
        "status")
            show_status
            ;;
        "logs")
            if [ -z "$service_name" ]; then
                echo -e "${RED}[ERROR]${NC} Please specify a service name for logs"
                echo "Available services: ${!SERVICES[@]}"
                exit 1
            fi
            show_logs "$service_name"
            ;;
        "help"|"-h"|"--help")
            show_usage
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown command: $command"
            echo
            show_usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
