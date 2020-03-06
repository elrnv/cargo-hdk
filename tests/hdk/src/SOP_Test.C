#include "SOP_Test.h"

// Needed for template generation with the ds file.
#include "SOP_Test.proto.h"

// Required for proper loading.
#include <UT/UT_DSOVersion.h>

#include <UT/UT_Interrupt.h>
#include <UT/UT_StringHolder.h>
#include <PRM/PRM_Include.h>
#include <PRM/PRM_TemplateBuilder.h>
#include <OP/OP_Operator.h>
#include <OP/OP_OperatorTable.h>

const UT_StringHolder SOP_Test::theSOPTypeName("hdk_test"_sh);

// Register sop operator
void
newSopOperator(OP_OperatorTable *table)
{
    table->addOperator(new OP_Operator(
                SOP_Test::theSOPTypeName,   // Internal name
                "Test",                     // UI name
                SOP_Test::myConstructor,    // How to build the SOP
                SOP_Test::buildTemplates(), // My parameters
                1,                          // Min # of sources
                1,                          // Max # of sources
                nullptr,                    // Local variables
                OP_FLAG_GENERATOR));        // Flag it as generator
}

static const char *theDsFile = R"THEDSFILE(
{
    name test
}
)THEDSFILE";


PRM_Template *
SOP_Test::buildTemplates()
{
    static PRM_TemplateBuilder templ("SOP_Test.C"_sh, theDsFile);
    return templ.templates();
}

class SOP_TestVerb : public SOP_NodeVerb
{
    public:
        SOP_TestVerb() {}
        virtual ~SOP_TestVerb() {}

        virtual SOP_NodeParms *allocParms() const { return new SOP_TestParms(); }
        virtual UT_StringHolder name() const { return SOP_Test::theSOPTypeName; }

        virtual CookMode cookMode(const SOP_NodeParms *parms) const { return COOK_GENERATOR; }

        virtual void cook(const CookParms &cookparms) const;

        static const SOP_NodeVerb::Register<SOP_TestVerb> theVerb;
};

const SOP_NodeVerb::Register<SOP_TestVerb> SOP_TestVerb::theVerb;

const SOP_NodeVerb *
SOP_Test::cookVerb() const
{
    return SOP_TestVerb::theVerb.get();
}

// Entry point to the SOP
void
SOP_TestVerb::cook(const SOP_NodeVerb::CookParms &cookparms) const
{
}
