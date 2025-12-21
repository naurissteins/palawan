# Function to ask a yes/no question
ask_yes_no() {
    while true;
 do
        read -p "$1 [y/n]: " yn </dev/tty
        case $yn in
            [Yy]* ) return 0;; # Yes
            [Nn]* ) return 1;; # No
            * ) echo "Please answer yes or no.";;
        esac
    done
}
