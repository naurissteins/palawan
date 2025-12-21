# Function to ask a yes/no question
ask_yes_no() {
    stop_log_output

    while true; do
        read -p "$1 [y/n]: " yn
        case $yn in
            [Yy]* )
                start_log_output
                return 0;; # Yes
            [Nn]* )
                start_log_output
                return 1;; # No
            * ) echo "Please answer yes or no.";;
        esac
    done
}
