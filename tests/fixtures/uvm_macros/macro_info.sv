class info_demo extends uvm_component;

    `uvm_component_utils(info_demo)

    function new(string name = "info_demo", uvm_component parent = null);
        super.new(name, parent);
    endfunction

    virtual task run_phase(uvm_phase phase);
        `uvm_info("RUN", "Starting run phase", UVM_LOW)
        `uvm_warning("WARN", "This is a warning message")
        `uvm_error("ERR", "This is an error message")
        // `uvm_fatal intentionally omitted to avoid simulation abort
    endtask

endclass
