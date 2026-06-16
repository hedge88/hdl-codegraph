class my_test extends uvm_test;

    `uvm_component_utils(my_test)

    my_env env;

    function new(string name = "my_test", uvm_component parent = null);
        super.new(name, parent);
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);

        // Type override
        my_driver::type_id::set_type_override(my_fast_driver::get_type());

        // Instance override
        my_driver::type_id::set_inst_override("my_fast_driver", "env.agent.drv");

        env = my_env::type_id::create("env", this);

        // Config DB set
        uvm_config_db#(virtual my_if)::set(this, "env.agent.*", "vif", top_if);
    endfunction

    virtual task run_phase(uvm_phase phase);
        my_sequence seq;

        phase.raise_objection(this);
        seq = my_sequence::type_id::create("seq");
        seq.start(env.agent.sqr);
        phase.drop_objection(this);
    endtask

endclass
