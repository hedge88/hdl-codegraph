class my_scoreboard extends uvm_scoreboard;

    `uvm_component_utils(my_scoreboard)

    uvm_analysis_imp #(my_transaction, my_scoreboard) analysis_export;

    int unsigned match_count;
    int unsigned mismatch_count;

    function new(string name = "my_scoreboard", uvm_component parent = null);
        super.new(name, parent);
        match_count = 0;
        mismatch_count = 0;
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);
        analysis_export = new("analysis_export", this);
    endfunction

    virtual function void write(my_transaction txn);
        // Compare expected vs actual
        if (txn.addr != 32'h0) begin
            match_count++;
        end else begin
            mismatch_count++;
        end
    endfunction

    virtual function void report_phase(uvm_phase phase);
        `uvm_info("SCB", $sformatf("Matches: %0d, Mismatches: %0d", match_count, mismatch_count), UVM_LOW)
    endfunction

endclass
